//! Typed FFmpeg filter graph compilation.
//!
//! The compiler accepts validated timeline values, converts optional v2 effect
//! payloads into a closed enum, and emits one `-filter_complex` argument. Raw
//! filter fragments from project files are never accepted.

use crate::{Clip, ClipEffect, Result, Timeline, TimelineTrackKind, YtpmError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterInputKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterInput {
    pub input_index: usize,
    pub clip_id: String,
    pub relative_path: String,
    pub kind: FilterInputKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderEffect {
    ColorAdjust {
        brightness: f64,
        contrast: f64,
        saturation: f64,
        gamma: f64,
    },
    Blur {
        radius: f64,
    },
    Sharpen {
        amount: f64,
    },
    Vignette {
        angle: f64,
    },
    ChromaKey {
        color: u32,
        similarity: f64,
        blend: f64,
    },
    FadeIn {
        duration_ms: u64,
    },
    FadeOut {
        duration_ms: u64,
    },
    Transform {
        x: f64,
        y: f64,
        scale: f64,
        rotation_degrees: f64,
        opacity: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFilterGraph {
    pub filter_complex: String,
    pub inputs: Vec<FilterInput>,
    pub video_output_label: String,
    pub audio_output_label: Option<String>,
}

/// Compiles a timeline into a deterministic, typed FFmpeg filter graph.
///
/// Input index zero is reserved for the generated black canvas. Media inputs
/// are returned in the exact order in which callers must append `-i` argv
/// entries. `burn_ass` only enables a fixed work-directory filename; no path
/// from the user is interpolated into the graph.
pub fn compile_filter_graph(timeline: &Timeline, burn_ass: bool) -> Result<CompiledFilterGraph> {
    if timeline.duration_ms == 0 {
        return Err(YtpmError::InvalidInput(
            "timeline duration_ms 必須大於 0 才能匯出".into(),
        ));
    }
    let width = timeline.output.width;
    let height = timeline.output.height;
    if width == 0 || height == 0 || !timeline.output.frame_rate.is_finite() {
        return Err(YtpmError::InvalidInput("輸出尺寸或 frame rate 無效".into()));
    }

    let mut graph = Vec::new();
    let mut inputs = Vec::new();
    let duration = seconds(timeline.duration_ms);
    graph.push(format!(
        "[0:v:0]trim=duration={duration},setpts=PTS-STARTPTS,format=rgba[base0]"
    ));

    let mut current_video = "base0".to_owned();
    let mut video_number = 0usize;
    let mut audio_labels = Vec::new();
    let mut input_index = 1usize;

    for track in &timeline.tracks {
        match track.kind {
            TimelineTrackKind::Video => {
                for clip in &track.clips {
                    let effects = effects_from_clip(clip)?;
                    let transform = transform_summary(&effects)?;
                    let clip_duration = clip.out_ms.checked_sub(clip.in_ms).ok_or_else(|| {
                        YtpmError::InvalidInput(format!("clip {} trim 範圍無效", clip.id))
                    })?;
                    if clip_duration == 0 {
                        return Err(YtpmError::InvalidInput(format!(
                            "clip {} duration 必須大於 0",
                            clip.id
                        )));
                    }
                    inputs.push(FilterInput {
                        input_index,
                        clip_id: clip.id.clone(),
                        relative_path: clip.relative_path.clone(),
                        kind: FilterInputKind::Video,
                    });
                    let clip_label = format!("vclip{video_number}");
                    let mut chain = vec![
                        format!(
                            "[{input_index}:v:0]trim=start={}:end={}",
                            seconds(clip.in_ms),
                            seconds(clip.out_ms)
                        ),
                        "setpts=PTS-STARTPTS".into(),
                        format!("scale={width}:{height}:force_original_aspect_ratio=decrease"),
                        format!("pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=black@0"),
                        "setsar=1".into(),
                        format!("fps={}", number(timeline.output.frame_rate)?),
                        format!(
                            "tpad=stop_mode=clone:stop_duration={}",
                            seconds(clip_duration)
                        ),
                        "format=rgba".into(),
                    ];
                    append_effect_filters(&mut chain, &effects, clip_duration)?;
                    append_transition_filters(&mut chain, clip.transition.as_ref(), clip_duration)?;
                    chain.push(format!("setpts=PTS+{}/TB", seconds(clip.start_ms)));
                    graph.push(format!("{}[{clip_label}]", chain.join(",")));

                    let next_video = format!("base{}", video_number + 1);
                    let start = seconds(clip.start_ms);
                    let end = seconds(clip.start_ms.saturating_add(clip_duration));
                    graph.push(format!(
                        "[{current_video}][{clip_label}]overlay=x={}:y={}:eof_action=pass:shortest=0:repeatlast=0:enable=between(t\\,{start}\\,{end})[{next_video}]",
                        number(transform.x)?,
                        number(transform.y)?
                    ));
                    current_video = next_video;
                    video_number += 1;
                    input_index += 1;
                }
            }
            TimelineTrackKind::Audio => {
                for clip in &track.clips {
                    let clip_duration = clip.out_ms.checked_sub(clip.in_ms).ok_or_else(|| {
                        YtpmError::InvalidInput(format!("clip {} trim 範圍無效", clip.id))
                    })?;
                    inputs.push(FilterInput {
                        input_index,
                        clip_id: clip.id.clone(),
                        relative_path: clip.relative_path.clone(),
                        kind: FilterInputKind::Audio,
                    });
                    let label = format!("aclip{}", audio_labels.len());
                    let volume = if clip.muted {
                        0.0
                    } else {
                        f64::from(clip.volume)
                    };
                    if !volume.is_finite() || !(0.0..=8.0).contains(&volume) {
                        return Err(YtpmError::InvalidInput(format!(
                            "clip {} volume 必須介於 0 與 8",
                            clip.id
                        )));
                    }
                    let mut chain = vec![
                        format!(
                            "[{input_index}:a:0]atrim=start={}:end={}",
                            seconds(clip.in_ms),
                            seconds(clip.out_ms)
                        ),
                        "asetpts=PTS-STARTPTS".into(),
                        format!("volume={}", number(volume)?),
                    ];
                    append_audio_transition_filters(
                        &mut chain,
                        clip.transition.as_ref(),
                        clip_duration,
                    )?;
                    chain.push(format!("adelay={}:all=1", clip.start_ms));
                    graph.push(format!("{}[{label}]", chain.join(",")));
                    audio_labels.push(label);
                    input_index += 1;
                }
            }
            TimelineTrackKind::Subtitle | TimelineTrackKind::Other => {}
        }
    }

    let video_output_label = if burn_ass {
        graph.push(format!(
            "[{current_video}]subtitles=filename='burn-subtitles.ass':charenc=UTF-8[vout]"
        ));
        "vout".to_owned()
    } else {
        current_video
    };
    let audio_output_label = if audio_labels.is_empty() {
        None
    } else {
        let label = "aout".to_owned();
        graph.push(format!(
            "{}amix=inputs={}:duration=longest:dropout_transition=0:normalize=0,atrim=duration={duration},asetpts=PTS-STARTPTS[{label}]",
            audio_labels
                .iter()
                .map(|item| format!("[{item}]"))
                .collect::<String>(),
            audio_labels.len()
        ));
        Some(label)
    };

    Ok(CompiledFilterGraph {
        filter_complex: graph.join(";"),
        inputs,
        video_output_label,
        audio_output_label,
    })
}

#[derive(Debug, Clone, Copy)]
struct TransformSummary {
    x: f64,
    y: f64,
}

fn transform_summary(effects: &[RenderEffect]) -> Result<TransformSummary> {
    let mut summary = TransformSummary { x: 0.0, y: 0.0 };
    for effect in effects {
        if let RenderEffect::Transform { x, y, .. } = effect {
            summary.x += *x;
            summary.y += *y;
        }
    }
    number(summary.x)?;
    number(summary.y)?;
    Ok(summary)
}

fn append_effect_filters(
    chain: &mut Vec<String>,
    effects: &[RenderEffect],
    clip_duration_ms: u64,
) -> Result<()> {
    for effect in effects {
        match effect {
            RenderEffect::ColorAdjust {
                brightness,
                contrast,
                saturation,
                gamma,
            } => chain.push(format!(
                "eq=brightness={}:contrast={}:saturation={}:gamma={}",
                ranged(*brightness, -1.0, 1.0, "brightness")?,
                ranged(*contrast, 0.0, 4.0, "contrast")?,
                ranged(*saturation, 0.0, 4.0, "saturation")?,
                ranged(*gamma, 0.1, 10.0, "gamma")?
            )),
            RenderEffect::Blur { radius } => chain.push(format!(
                "gblur=sigma={}",
                ranged(*radius, 0.0, 100.0, "blur radius")?
            )),
            RenderEffect::Sharpen { amount } => chain.push(format!(
                "unsharp=5:5:{}:5:5:0",
                ranged(*amount, -2.0, 5.0, "sharpen amount")?
            )),
            RenderEffect::Vignette { angle } => chain.push(format!(
                "vignette=angle={}",
                ranged(*angle, 0.0, 2.0, "vignette angle")?
            )),
            RenderEffect::ChromaKey {
                color,
                similarity,
                blend,
            } => chain.push(format!(
                "chromakey=0x{color:06X}:{}:{}",
                ranged(*similarity, 0.01, 1.0, "chroma similarity")?,
                ranged(*blend, 0.0, 1.0, "chroma blend")?
            )),
            RenderEffect::FadeIn { duration_ms } => {
                let duration = (*duration_ms).min(clip_duration_ms);
                chain.push(format!("fade=t=in:st=0:d={}:alpha=1", seconds(duration)));
            }
            RenderEffect::FadeOut { duration_ms } => {
                let duration = (*duration_ms).min(clip_duration_ms);
                let start = clip_duration_ms.saturating_sub(duration);
                chain.push(format!(
                    "fade=t=out:st={}:d={}:alpha=1",
                    seconds(start),
                    seconds(duration)
                ));
            }
            RenderEffect::Transform {
                scale,
                rotation_degrees,
                opacity,
                ..
            } => {
                chain.push(format!(
                    "scale=iw*{}:ih*{}",
                    ranged(*scale, 0.05, 8.0, "transform scale")?,
                    ranged(*scale, 0.05, 8.0, "transform scale")?
                ));
                let rotation = ranged(*rotation_degrees, -3600.0, 3600.0, "transform rotation")?;
                chain.push(format!(
                    "rotate={rotation}*PI/180:ow=rotw(iw):oh=roth(ih):c=none@0"
                ));
                chain.push(format!(
                    "colorchannelmixer=aa={}",
                    ranged(*opacity, 0.0, 1.0, "transform opacity")?
                ));
            }
        }
    }
    Ok(())
}

fn append_transition_filters(
    chain: &mut Vec<String>,
    transition: Option<&crate::timeline_service::Transition>,
    clip_duration_ms: u64,
) -> Result<()> {
    let Some(transition) = transition else {
        return Ok(());
    };
    let duration = transition.duration_ms.min(clip_duration_ms);
    match transition.kind.as_str() {
        "cut" | "none" => Ok(()),
        "fade" | "fade_in" | "dissolve" | "crossfade" => {
            chain.push(format!("fade=t=in:st=0:d={}:alpha=1", seconds(duration)));
            Ok(())
        }
        "fade_out" => {
            chain.push(format!(
                "fade=t=out:st={}:d={}:alpha=1",
                seconds(clip_duration_ms.saturating_sub(duration)),
                seconds(duration)
            ));
            Ok(())
        }
        other => Err(YtpmError::InvalidInput(format!(
            "不支援的 transition kind：{other}"
        ))),
    }
}

fn append_audio_transition_filters(
    chain: &mut Vec<String>,
    transition: Option<&crate::timeline_service::Transition>,
    clip_duration_ms: u64,
) -> Result<()> {
    let Some(transition) = transition else {
        return Ok(());
    };
    let duration = transition.duration_ms.min(clip_duration_ms);
    match transition.kind.as_str() {
        "cut" | "none" => Ok(()),
        "fade" | "fade_in" | "dissolve" | "crossfade" => {
            chain.push(format!("afade=t=in:st=0:d={}", seconds(duration)));
            Ok(())
        }
        "fade_out" => {
            chain.push(format!(
                "afade=t=out:st={}:d={}",
                seconds(clip_duration_ms.saturating_sub(duration)),
                seconds(duration)
            ));
            Ok(())
        }
        other => Err(YtpmError::InvalidInput(format!(
            "不支援的 audio transition kind：{other}"
        ))),
    }
}

fn effects_from_clip(clip: &Clip) -> Result<Vec<RenderEffect>> {
    clip.effects.iter().map(RenderEffect::try_from).collect()
}

impl TryFrom<&ClipEffect> for RenderEffect {
    type Error = YtpmError;

    fn try_from(effect: &ClipEffect) -> Result<Self> {
        Ok(match effect {
            ClipEffect::ColorAdjust {
                brightness,
                contrast,
                saturation,
                gamma,
            } => Self::ColorAdjust {
                brightness: f64::from(*brightness),
                contrast: f64::from(*contrast),
                saturation: f64::from(*saturation),
                gamma: f64::from(*gamma),
            },
            ClipEffect::Blur { radius } => Self::Blur {
                radius: f64::from(*radius),
            },
            ClipEffect::Sharpen { amount } => Self::Sharpen {
                amount: f64::from(*amount),
            },
            ClipEffect::Vignette { angle } => Self::Vignette {
                angle: f64::from(*angle),
            },
            ClipEffect::ChromaKey {
                color,
                similarity,
                blend,
            } => Self::ChromaKey {
                color: parse_color(color)?,
                similarity: f64::from(*similarity),
                blend: f64::from(*blend),
            },
            ClipEffect::FadeIn { duration_ms } => Self::FadeIn {
                duration_ms: *duration_ms,
            },
            ClipEffect::FadeOut { duration_ms } => Self::FadeOut {
                duration_ms: *duration_ms,
            },
            ClipEffect::Transform {
                x,
                y,
                scale,
                rotation_degrees,
                opacity,
            } => Self::Transform {
                x: f64::from(*x),
                y: f64::from(*y),
                scale: f64::from(*scale),
                rotation_degrees: f64::from(*rotation_degrees),
                opacity: f64::from(*opacity),
            },
        })
    }
}

fn parse_color(value: &str) -> Result<u32> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(YtpmError::InvalidInput(
            "chroma key color 必須是 #RRGGBB".into(),
        ));
    }
    u32::from_str_radix(digits, 16)
        .map_err(|_| YtpmError::InvalidInput("chroma key color 無效".into()))
}

fn ranged(value: f64, min: f64, max: f64, name: &str) -> Result<String> {
    if !value.is_finite() || value < min || value > max {
        return Err(YtpmError::InvalidInput(format!(
            "{name} 必須介於 {min} 與 {max}"
        )));
    }
    number(value)
}

fn number(value: f64) -> Result<String> {
    if !value.is_finite() {
        return Err(YtpmError::InvalidInput(
            "filter 數值不可為 NaN/Infinity".into(),
        ));
    }
    let mut rendered = format!("{value:.6}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.push('0');
    }
    Ok(rendered)
}

fn seconds(value_ms: u64) -> String {
    let seconds = value_ms / 1_000;
    let millis = value_ms % 1_000;
    format!("{seconds}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Clip, Timeline};

    #[test]
    fn compiles_multitrack_placement_without_raw_shell_fragments() {
        let mut timeline = Timeline {
            duration_ms: 2_000,
            ..Timeline::default()
        };
        timeline.tracks[0].clips.push(clip("05_video/a.mp4", 500));
        timeline.tracks[1].clips.push(clip("03_voice/a.wav", 250));
        let graph = compile_filter_graph(&timeline, true).expect("compile graph");
        assert_eq!(graph.inputs.len(), 2);
        assert!(graph.filter_complex.contains("overlay=x=0.0:y=0.0"));
        assert!(graph.filter_complex.contains("adelay=250:all=1"));
        assert!(graph.filter_complex.contains("amix=inputs=1"));
        assert!(graph
            .filter_complex
            .contains("subtitles=filename='burn-subtitles.ass'"));
        assert!(!graph.filter_complex.contains("05_video/a.mp4"));
    }

    #[test]
    fn compiles_only_whitelisted_typed_effects() {
        let effect = RenderEffect::try_from(&ClipEffect::ChromaKey {
            color: "#00FF00".into(),
            similarity: 0.2,
            blend: 0.05,
        })
        .expect("typed effect");
        let RenderEffect::ChromaKey {
            color,
            similarity,
            blend,
        } = effect
        else {
            panic!("expected chroma key");
        };
        assert_eq!(color, 0x00ff00);
        assert!((similarity - 0.2).abs() < 0.000_001);
        assert!((blend - 0.05).abs() < 0.000_001);
        assert!(RenderEffect::try_from(&ClipEffect::ChromaKey {
            color: "green,subtitles=/tmp/evil".into(),
            similarity: 0.2,
            blend: 0.05,
        })
        .is_err());
    }

    fn clip(path: &str, start_ms: u64) -> Clip {
        Clip {
            id: uuid::Uuid::new_v4().to_string(),
            asset_id: uuid::Uuid::new_v4().to_string(),
            relative_path: path.into(),
            label: "clip".into(),
            start_ms,
            in_ms: 0,
            out_ms: 1_000,
            duration_ms: 1_000,
            volume: 1.0,
            muted: false,
            transition: None,
            effects: Vec::new(),
        }
    }
}
