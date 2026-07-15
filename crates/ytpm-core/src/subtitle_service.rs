//! SRT/WebVTT parsing and deterministic ASS subtitle generation.

use crate::media_service::{create_dir_all_checked, reject_reparse_points, resolve_project_file};
use crate::timeline_service::{replace_file, validate};
use crate::{Result, SubtitleStyle, Timeline, TimelineTrackKind, YtpmError};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssStyle {
    pub font_family: String,
    pub font_size: f64,
    pub primary_color: String,
    pub outline_color: String,
    pub background_color: String,
    pub bold: bool,
    pub italic: bool,
    pub outline: f64,
    pub shadow: f64,
    pub alignment: u8,
    pub margin_left: u32,
    pub margin_right: u32,
    pub margin_vertical: u32,
}

impl Default for AssStyle {
    fn default() -> Self {
        Self {
            font_family: "Microsoft JhengHei UI".into(),
            font_size: 48.0,
            primary_color: "#FFFFFFFF".into(),
            outline_color: "#000000FF".into(),
            background_color: "#00000000".into(),
            bold: true,
            italic: false,
            outline: 2.0,
            shadow: 1.0,
            alignment: 2,
            margin_left: 40,
            margin_right: 40,
            margin_vertical: 48,
        }
    }
}

/// Converts all subtitle-track SRT/VTT clips into one ASS file.
///
/// Returns `false` without creating the file when the timeline has no visible
/// cues. Source paths still pass through the project-relative/reparse checks.
pub fn render_timeline_ass(project_dir: &Path, timeline: &Timeline, output: &Path) -> Result<bool> {
    validate(timeline)?;
    reject_reparse_points(project_dir)?;
    reject_reparse_points(output)?;
    let style = AssStyle::from(&timeline.output.subtitle_style);
    let mut cues = Vec::new();

    for track in timeline
        .tracks
        .iter()
        .filter(|track| track.kind == TimelineTrackKind::Subtitle)
    {
        for clip in &track.clips {
            if clip.muted {
                continue;
            }
            let source = resolve_project_file(project_dir, &clip.relative_path)?;
            let content =
                fs::read_to_string(&source).map_err(|error| YtpmError::io(&source, error))?;
            let extension = source
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let source_cues = match extension.as_str() {
                "srt" => parse_srt(&content)?,
                "vtt" => parse_webvtt(&content)?,
                _ => {
                    return Err(YtpmError::InvalidInput(format!(
                        "字幕僅支援 .srt 或 .vtt：{}",
                        clip.relative_path
                    )))
                }
            };
            cues.extend(place_cues(
                source_cues,
                clip.start_ms,
                clip.in_ms,
                clip.out_ms,
                timeline.duration_ms,
            ));
        }
    }
    if cues.is_empty() {
        return Ok(false);
    }
    cues.sort_by_key(|cue| (cue.start_ms, cue.end_ms));
    if let Some(parent) = output.parent() {
        create_dir_all_checked(parent)?;
    }
    let document = build_ass(timeline.output.width, timeline.output.height, &style, &cues)?;
    atomic_write_ass(output, document.as_bytes())?;
    Ok(true)
}

fn atomic_write_ass(output: &Path, bytes: &[u8]) -> Result<()> {
    reject_reparse_points(output)?;
    if let Ok(metadata) = fs::symlink_metadata(output) {
        if super::media_service::metadata_is_reparse_point(&metadata) {
            return Err(YtpmError::InvalidInput(format!(
                "拒絕覆寫 symlink/junction/reparse ASS：{}",
                output.display()
            )));
        }
    }
    let parent = output.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 ASS parent：{}", output.display()))
    })?;
    let file_name = output
        .file_name()
        .ok_or_else(|| YtpmError::InvalidInput("ASS 輸出檔名無效".into()))?
        .to_string_lossy();
    let temporary = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result: Result<()> = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| YtpmError::io(&temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| YtpmError::io(&temporary, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temporary, error))?;
        replace_file(&temporary, output).map_err(|error| YtpmError::io(output, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn parse_srt(content: &str) -> Result<Vec<SubtitleCue>> {
    parse_caption_blocks(content, false)
}

pub fn parse_webvtt(content: &str) -> Result<Vec<SubtitleCue>> {
    parse_caption_blocks(content, true)
}

fn parse_caption_blocks(content: &str, webvtt: bool) -> Result<Vec<SubtitleCue>> {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();
    for raw_block in normalized.split("\n\n") {
        let lines = raw_block.lines().map(str::trim_end).collect::<Vec<_>>();
        if lines.is_empty() || lines.iter().all(|line| line.trim().is_empty()) {
            continue;
        }
        let first = lines[0].trim_start_matches('\u{feff}').trim();
        if webvtt
            && (first.eq_ignore_ascii_case("WEBVTT")
                || first.starts_with("NOTE")
                || first.starts_with("STYLE")
                || first.starts_with("REGION"))
        {
            continue;
        }
        let timing_index = lines
            .iter()
            .position(|line| line.contains("-->"))
            .ok_or_else(|| YtpmError::InvalidInput(format!("字幕區塊缺少 -->：{first}")))?;
        let (start_ms, end_ms) = parse_timing_line(lines[timing_index])?;
        let text = lines
            .iter()
            .skip(timing_index + 1)
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned();
        if text.is_empty() {
            continue;
        }
        cues.push(SubtitleCue {
            start_ms,
            end_ms,
            text,
        });
    }
    Ok(cues)
}

fn parse_timing_line(line: &str) -> Result<(u64, u64)> {
    let mut sides = line.splitn(2, "-->");
    let start = sides.next().unwrap_or_default().trim();
    let end = sides
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default();
    let start_ms = parse_timestamp(start)?;
    let end_ms = parse_timestamp(end)?;
    if end_ms <= start_ms {
        return Err(YtpmError::InvalidInput(format!(
            "字幕結束時間必須晚於開始時間：{line}"
        )));
    }
    Ok((start_ms, end_ms))
}

fn parse_timestamp(value: &str) -> Result<u64> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err(YtpmError::InvalidInput(format!(
            "字幕時間格式無效：{value}"
        )));
    }
    let (hours, minutes, seconds_part) = if parts.len() == 3 {
        (
            parse_u64(parts[0], value)?,
            parse_u64(parts[1], value)?,
            parts[2],
        )
    } else {
        (0, parse_u64(parts[0], value)?, parts[1])
    };
    let mut seconds_fields = seconds_part.splitn(2, '.');
    let seconds = parse_u64(seconds_fields.next().unwrap_or_default(), value)?;
    let fraction = seconds_fields.next().unwrap_or("0");
    if minutes > 59
        || seconds > 59
        || fraction.len() > 3
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(YtpmError::InvalidInput(format!(
            "字幕時間格式無效：{value}"
        )));
    }
    let milliseconds = match fraction.len() {
        0 => 0,
        1 => parse_u64(fraction, value)? * 100,
        2 => parse_u64(fraction, value)? * 10,
        _ => parse_u64(fraction, value)?,
    };
    hours
        .checked_mul(3_600_000)
        .and_then(|total| total.checked_add(minutes * 60_000))
        .and_then(|total| total.checked_add(seconds * 1_000))
        .and_then(|total| total.checked_add(milliseconds))
        .ok_or_else(|| YtpmError::InvalidInput(format!("字幕時間超出範圍：{value}")))
}

fn parse_u64(value: &str, original: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| YtpmError::InvalidInput(format!("字幕時間格式無效：{original}")))
}

fn place_cues(
    cues: Vec<SubtitleCue>,
    clip_start_ms: u64,
    clip_in_ms: u64,
    clip_out_ms: u64,
    timeline_duration_ms: u64,
) -> Vec<SubtitleCue> {
    cues.into_iter()
        .filter_map(|cue| {
            let source_start = cue.start_ms.max(clip_in_ms);
            let source_end = cue.end_ms.min(clip_out_ms);
            if source_end <= source_start {
                return None;
            }
            let start_ms = clip_start_ms.saturating_add(source_start - clip_in_ms);
            let end_ms = clip_start_ms.saturating_add(source_end - clip_in_ms);
            let clamped_end = end_ms.min(timeline_duration_ms);
            (clamped_end > start_ms).then_some(SubtitleCue {
                start_ms,
                end_ms: clamped_end,
                text: cue.text,
            })
        })
        .collect()
}

fn build_ass(width: u32, height: u32, style: &AssStyle, cues: &[SubtitleCue]) -> Result<String> {
    if width == 0 || height == 0 {
        return Err(YtpmError::InvalidInput("ASS 輸出尺寸不可為 0".into()));
    }
    if !style.font_size.is_finite()
        || !(1.0..=300.0).contains(&style.font_size)
        || !style.outline.is_finite()
        || !(0.0..=20.0).contains(&style.outline)
        || !style.shadow.is_finite()
        || !(0.0..=20.0).contains(&style.shadow)
        || !(1..=9).contains(&style.alignment)
    {
        return Err(YtpmError::InvalidInput("字幕樣式數值超出允許範圍".into()));
    }
    let font = escape_ass_field(&style.font_family)?;
    let primary = css_to_ass_color(&style.primary_color)?;
    let outline_color = css_to_ass_color(&style.outline_color)?;
    let background = css_to_ass_color(&style.background_color)?;
    let bold = if style.bold { -1 } else { 0 };
    let italic = if style.italic { -1 } else { 0 };
    let mut output = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {width}\nPlayResY: {height}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,{font},{},{primary},{primary},{outline_color},{background},{bold},{italic},0,0,100,100,0,0,1,{},{},{},{},{},{},1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        style.font_size,
        style.outline,
        style.shadow,
        style.alignment,
        style.margin_left,
        style.margin_right,
        style.margin_vertical
    );
    for cue in cues {
        output.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            ass_timestamp(cue.start_ms),
            ass_timestamp(cue.end_ms),
            escape_ass_text(&cue.text)
        ));
    }
    Ok(output)
}

impl From<&SubtitleStyle> for AssStyle {
    fn from(style: &SubtitleStyle) -> Self {
        Self {
            font_family: style.font_family.clone(),
            font_size: f64::from(style.font_size),
            primary_color: style.primary_color.clone(),
            outline_color: style.outline_color.clone(),
            background_color: style.background_color.clone(),
            bold: style.bold,
            italic: style.italic,
            outline: f64::from(style.outline_width),
            shadow: f64::from(style.shadow_depth),
            alignment: style.alignment,
            margin_left: style.margin_left,
            margin_right: style.margin_right,
            margin_vertical: style.margin_vertical,
        }
    }
}

fn escape_ass_field(value: &str) -> Result<String> {
    if value.trim().is_empty()
        || value
            .chars()
            .any(|character| matches!(character, ',' | '\r' | '\n'))
    {
        return Err(YtpmError::InvalidInput(
            "字幕字型名稱不可為空或包含逗號/換行".into(),
        ));
    }
    Ok(value.to_owned())
}

fn escape_ass_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace("\r\n", "\\N")
        .replace(['\r', '\n'], "\\N")
}

fn css_to_ass_color(value: &str) -> Result<String> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(YtpmError::InvalidInput(
            "字幕顏色必須是 #RRGGBB 或 #RRGGBBAA".into(),
        ));
    }
    let red = &digits[0..2];
    let green = &digits[2..4];
    let blue = &digits[4..6];
    let ass_alpha = if digits.len() == 8 {
        let css_alpha = u8::from_str_radix(&digits[6..8], 16)
            .map_err(|_| YtpmError::InvalidInput("字幕 alpha 無效".into()))?;
        format!("{:02X}", 255 - css_alpha)
    } else {
        "00".to_owned()
    };
    Ok(format!(
        "&H{ass_alpha}{}{}{}",
        blue.to_ascii_uppercase(),
        green.to_ascii_uppercase(),
        red.to_ascii_uppercase()
    ))
}

fn ass_timestamp(value_ms: u64) -> String {
    let centiseconds = value_ms / 10;
    let hours = centiseconds / 360_000;
    let minutes = (centiseconds / 6_000) % 60;
    let seconds = (centiseconds / 100) % 60;
    let fraction = centiseconds % 100;
    format!("{hours}:{minutes:02}:{seconds:02}.{fraction:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimelineClip;

    #[test]
    fn parses_srt_and_webvtt_timestamps() {
        let srt = "1\r\n00:00:01,250 --> 00:00:02,500\r\n第一行\r\n第二行\r\n";
        let cues = parse_srt(srt).expect("parse srt");
        assert_eq!(cues[0].start_ms, 1_250);
        assert_eq!(cues[0].end_ms, 2_500);
        assert_eq!(cues[0].text, "第一行\n第二行");

        let vtt = "WEBVTT\n\nc1\n00:01.000 --> 00:02.250 position:50%\nHello\n";
        let cues = parse_webvtt(vtt).expect("parse vtt");
        assert_eq!(cues[0].start_ms, 1_000);
        assert_eq!(cues[0].end_ms, 2_250);
    }

    #[test]
    fn clips_offsets_and_escapes_ass_cues() {
        let placed = place_cues(
            vec![SubtitleCue {
                start_ms: 1_000,
                end_ms: 3_000,
                text: "{字幕}\\line\nnext".into(),
            }],
            5_000,
            1_500,
            2_500,
            10_000,
        );
        assert_eq!(placed[0].start_ms, 5_000);
        assert_eq!(placed[0].end_ms, 6_000);
        let ass = build_ass(1920, 1080, &AssStyle::default(), &placed).expect("build ass");
        assert!(ass.contains("Dialogue: 0,0:00:05.00,0:00:06.00"));
        assert!(ass.contains("\\{字幕\\}\\\\line\\Nnext"));
    }

    #[test]
    fn rejects_invalid_color_and_timing() {
        assert!(css_to_ass_color("red,subtitles=/tmp/evil").is_err());
        assert!(parse_srt("1\n00:00:02,000 --> 00:00:01,000\nNo\n").is_err());
    }

    #[test]
    fn public_ass_writer_validates_timeline_and_atomically_replaces_output() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("captions.srt"),
            "1\n00:00:00,000 --> 00:00:00,500\nHello\n",
        )
        .unwrap();
        let mut timeline = Timeline {
            duration_ms: 1_000,
            ..Default::default()
        };
        timeline.tracks[2].clips.push(TimelineClip {
            id: Uuid::new_v4().to_string(),
            asset_id: Uuid::new_v4().to_string(),
            relative_path: "captions.srt".into(),
            label: "captions".into(),
            start_ms: 0,
            in_ms: 0,
            out_ms: 1_000,
            duration_ms: 1_000,
            volume: 1.0,
            muted: false,
            transition: None,
            effects: Vec::new(),
        });
        let output = project.path().join("work/burn.ass");
        fs::create_dir(project.path().join("work")).unwrap();
        fs::write(&output, "old").unwrap();

        assert!(render_timeline_ass(project.path(), &timeline, &output).unwrap());
        assert!(fs::read_to_string(&output).unwrap().contains("Dialogue:"));
        assert!(fs::read_dir(output.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));

        let before = fs::read(&output).unwrap();
        timeline.output.format = "webm".into();
        assert!(render_timeline_ass(project.path(), &timeline, &output).is_err());
        assert_eq!(fs::read(&output).unwrap(), before);
    }
}
