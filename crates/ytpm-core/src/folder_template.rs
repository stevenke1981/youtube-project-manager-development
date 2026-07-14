pub fn expected_directories() -> &'static [&'static str] {
    &[
        "01_research/references",
        "02_script/prompts",
        "03_voice/raw",
        "03_voice/processed",
        "03_voice/music",
        "03_voice/sound_effects",
        "04_images/generated",
        "04_images/references",
        "04_images/characters",
        "04_images/backgrounds",
        "04_images/selected",
        "05_video/raw",
        "05_video/generated",
        "05_video/edited",
        "05_video/final",
        "06_subtitles/translations",
        "07_thumbnail/source",
        "07_thumbnail/drafts",
        "07_thumbnail/final",
        "08_metadata",
        "09_exports/review",
        "09_exports/upload",
        "10_archive",
    ]
}

pub fn template_files() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "README.md",
            "# {{TITLE}}\n\n由 YouTube Project Manager 建立。\n",
        ),
        ("01_research/links.md", "# 參考連結\n\n"),
        ("01_research/notes.md", "# 研究筆記\n\n"),
        ("02_script/outline.md", "# 影片大綱\n\n"),
        (
            "02_script/script.md",
            "# 完整腳本\n\n## 場景 01\n\n- 旁白：\n- 畫面：\n",
        ),
        ("02_script/prompts/README.md", "# 提示詞\n\n"),
        ("08_metadata/title.md", "# 標題候選\n\n"),
        ("08_metadata/description.md", "# 影片描述\n\n"),
        ("08_metadata/tags.txt", ""),
        ("08_metadata/chapters.txt", "00:00 開始\n"),
        ("08_metadata/pinned-comment.md", "# 置頂留言\n\n"),
        (
            "tasks.json",
            "{\n  \"schema_version\": 1,\n  \"tasks\": []\n}\n",
        ),
        (
            "assets.json",
            "{\n  \"schema_version\": 1,\n  \"assets\": []\n}\n",
        ),
        ("activity.log", ""),
    ]
}
