use crate::config::AppConfig;
use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::event::KeyCode;
use crossterm::queue;
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType};
use std::io::{self, Write};

use super::form::{
    kb_embedding_provider_value, parse_bool_field, parse_provider_model_choice,
    provider_model_choice_values, run_form, vision_provider_value, Field,
};
use super::input::read_key;
use super::ui::{display_width, draw_box, pad, truncate};

pub(crate) fn edit_plugins(stdout: &mut io::Stdout, config: &mut AppConfig) -> Result<()> {
    let mut selected = 0usize;
    loop {
        let count = plugin_names().len();
        draw_plugin_menu(stdout, config, selected)?;
        match read_key()? {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(count - 1),
            KeyCode::Char(' ') => toggle_plugin(config, selected),
            KeyCode::Enter | KeyCode::Char('i') => edit_plugin_detail(stdout, config, selected)?,
            _ => {}
        }
    }
}

fn draw_plugin_menu(stdout: &mut io::Stdout, config: &AppConfig, selected: usize) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let width = cols.saturating_sub(4).max(60);
    let height = rows.saturating_sub(2).max(10);
    let x = 2;
    let y = 1;
    queue!(stdout, Clear(ClearType::All))?;
    draw_box(stdout, x, y, width, height, " PLUGINS ")?;
    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        Print("[Space]启用/禁用 [Enter]配置 [j/k]移动 [q]返回")
    )?;
    queue!(
        stdout,
        MoveTo(x + 2, y + 3),
        SetAttribute(Attribute::Bold),
        Print(pad(
            &plugin_row("状态", "插件", "说明", width.saturating_sub(4) as usize),
            width.saturating_sub(4) as usize,
        )),
        SetAttribute(Attribute::Reset)
    )?;
    let plugins = plugin_names();
    let visible_rows = height.saturating_sub(6) as usize;
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    for row in 0..visible_rows {
        let index = start + row;
        if index >= plugins.len() {
            break;
        }
        let (_, name, description) = plugins[index];
        let state = if plugin_enabled(config, index) {
            "[ON]"
        } else {
            "[OFF]"
        };
        let line = plugin_row(state, name, description, width.saturating_sub(4) as usize);
        queue!(stdout, MoveTo(x + 2, y + row as u16 + 4))?;
        if index == selected {
            queue!(
                stdout,
                SetAttribute(Attribute::Reverse),
                Print(pad(&line, width.saturating_sub(4) as usize)),
                SetAttribute(Attribute::Reset)
            )?;
        } else {
            queue!(stdout, Print(pad(&line, width.saturating_sub(4) as usize)))?;
        }
    }
    stdout.flush()?;
    Ok(())
}

fn plugin_row(state: &str, name: &str, description: &str, width: usize) -> String {
    let fixed = pad(state, 8) + &pad(name, 24);
    let remaining = width.saturating_sub(display_width(&fixed)).max(10);
    fixed + &truncate(description, remaining)
}

fn plugin_names() -> [(&'static str, &'static str, &'static str); 13] {
    [
        ("web", "网络搜索", "搜索 API 与脚本 fallback"),
        ("deep_research", "深度研究", "长任务研究并输出 Markdown"),
        ("vision", "识图", "图片理解和终端预览"),
        ("image_generation", "生图", "文本生成图片"),
        ("web_images", "搜图", "网络图片搜索、下载与审核"),
        ("print_image", "打印图片", "终端图片打印尺寸"),
        ("memes", "表情包", "人格表情库与发送尺寸"),
        ("knowledge_base", "知识库", "本地文件检索与语义索引"),
        ("archlinux", "Arch Linux", "AUR 状态与 ArchWiki 查询"),
        ("man", "在线手册", "在线 man 手册搜索与读取"),
        ("memory", "记忆", "长期记忆与联想"),
        ("package_advisor", "AUR 审查", "PKGBUILD/AUR 安全审查"),
        (
            "linux_game_compatibility",
            "Linux 游戏兼容",
            "Proton/反作弊/兼容性查询",
        ),
    ]
}

fn plugin_enabled(config: &AppConfig, index: usize) -> bool {
    match index {
        0 => config.plugins.web.enabled,
        1 => config.plugins.deep_research.enabled,
        2 => config.plugins.vision.enabled,
        3 => config.plugins.image_generation.enabled,
        4 => config.plugins.web_images.enabled,
        5 => config.plugins.print_image.enabled,
        6 => config.plugins.memes.enabled,
        7 => config.plugins.knowledge_base.enabled,
        8 => config.plugins.archlinux.enabled,
        9 => config.plugins.man.enabled,
        10 => config.plugins.memory.enabled,
        11 => config.plugins.package_advisor.enabled,
        12 => config.plugins.linux_game_compatibility.enabled,
        _ => false,
    }
}

fn toggle_plugin(config: &mut AppConfig, index: usize) {
    let value = !plugin_enabled(config, index);
    match index {
        0 => config.plugins.web.enabled = value,
        1 => config.plugins.deep_research.enabled = value,
        2 => config.plugins.vision.enabled = value,
        3 => config.plugins.image_generation.enabled = value,
        4 => config.plugins.web_images.enabled = value,
        5 => config.plugins.print_image.enabled = value,
        6 => config.plugins.memes.enabled = value,
        7 => config.plugins.knowledge_base.enabled = value,
        8 => config.plugins.archlinux.enabled = value,
        9 => config.plugins.man.enabled = value,
        10 => config.plugins.memory.enabled = value,
        11 => config.plugins.package_advisor.enabled = value,
        12 => config.plugins.linux_game_compatibility.enabled = value,
        _ => {}
    }
}

fn edit_plugin_detail(stdout: &mut io::Stdout, config: &mut AppConfig, index: usize) -> Result<()> {
    let title = format!(" PLUGIN: {} ", plugin_names()[index].1);
    let mut fields = plugin_fields(config, index);
    if !run_form(stdout, &title, &mut fields)? {
        return Ok(());
    }
    apply_plugin_fields(config, index, &fields)
}

fn plugin_fields(config: &AppConfig, index: usize) -> Vec<Field> {
    match index {
        0 => vec![
            Field::boolean("启用", config.plugins.web.enabled),
            Field::textarea(
                "Tavily API Keys",
                config.plugins.web.tavily_api_keys.join("\n"),
            ),
            Field::textarea(
                "Firecrawl API Keys",
                config.plugins.web.firecrawl_api_keys.join("\n"),
            ),
            Field::textarea(
                "AnySearch API Keys",
                config.plugins.web.anysearch_api_keys.join("\n"),
            ),
            Field::new("SearXNG URL", config.plugins.web.searxng_base_url.clone()),
        ],
        1 => vec![
            Field::boolean("启用", config.plugins.deep_research.enabled),
            Field::new("输出目录", config.plugins.deep_research.output_dir.clone()),
            Field::new(
                "思考深度",
                config.plugins.deep_research.thinking_depth.clone(),
            )
            .choices(&["minimal", "low", "medium", "high", "xhigh"]),
            Field::new(
                "最大审视修正次数",
                config
                    .plugins
                    .deep_research
                    .max_review_revisions
                    .to_string(),
            ),
            Field::new(
                "每轮工具步数",
                config
                    .plugins
                    .deep_research
                    .max_tool_steps_per_round
                    .to_string(),
            ),
            Field::new(
                "最终字数上限",
                config
                    .plugins
                    .deep_research
                    .max_final_answer_chars
                    .to_string(),
            ),
            Field::new(
                "工具超时秒数",
                config
                    .plugins
                    .deep_research
                    .tool_call_timeout_seconds
                    .to_string(),
            ),
            Field::boolean("显示过程进度", config.plugins.deep_research.show_progress),
        ],
        2 => vec![
            Field::boolean("启用", config.plugins.vision.enabled),
            Field::boolean(
                "优先当前多模态模型",
                config.plugins.vision.prefer_current_multimodal_model,
            ),
            Field::new("识图 Provider/模型", vision_provider_value(config))
                .choices_owned(provider_model_choice_values(config, true)),
        ],
        3 => vec![
            Field::boolean("启用", config.plugins.image_generation.enabled),
            Field::new(
                "生图 API 类型",
                config.plugins.image_generation.provider_type.clone(),
            )
            .choices(&["openai", "rightcode"]),
            Field::new("Base URL", config.plugins.image_generation.base_url.clone()),
            Field::textarea(
                "API Keys",
                config.plugins.image_generation.api_keys.join("\n"),
            ),
            Field::new("模型", config.plugins.image_generation.model.clone()),
            Field::new(
                "默认宽高比",
                config.plugins.image_generation.default_aspect_ratio.clone(),
            )
            .choices(&[
                "自动", "1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9",
            ]),
            Field::new(
                "默认分辨率",
                config.plugins.image_generation.default_resolution.clone(),
            )
            .choices(&["1K", "2K", "4K"]),
            Field::new(
                "输出目录",
                config.plugins.image_generation.output_dir.clone(),
            ),
            Field::boolean("完成后打印", config.plugins.image_generation.auto_print),
            Field::new(
                "超时秒数",
                config.plugins.image_generation.timeout_seconds.to_string(),
            ),
        ],
        4 => vec![
            Field::boolean("启用", config.plugins.web_images.enabled),
            Field::boolean(
                "视觉模型审核",
                config.plugins.web_images.vision_screening_enabled,
            ),
            Field::new(
                "数量上限",
                config.plugins.web_images.max_results.to_string(),
            ),
            Field::boolean("安全搜索", config.plugins.web_images.safe_search),
            Field::boolean("自动预览", config.plugins.web_images.auto_preview),
            Field::new(
                "默认预览数量",
                config.plugins.web_images.preview_count.to_string(),
            ),
            Field::new(
                "最大下载 MB",
                config.plugins.web_images.max_download_mb.to_string(),
            ),
            Field::new(
                "超时秒数",
                config.plugins.web_images.timeout_seconds.to_string(),
            ),
        ],
        5 => vec![
            Field::boolean("启用", config.plugins.print_image.enabled),
            Field::new(
                "打印宽度百分比",
                config.plugins.print_image.width_percent.to_string(),
            ),
            Field::new(
                "打印高度百分比",
                config.plugins.print_image.height_percent.to_string(),
            ),
        ],
        6 => vec![
            Field::boolean("启用", config.plugins.memes.enabled),
            Field::new(
                "发送宽度百分比",
                config.plugins.memes.width_percent.to_string(),
            ),
            Field::new(
                "发送高度百分比",
                config.plugins.memes.height_percent.to_string(),
            ),
            Field::new("最大图片 MB", config.plugins.memes.max_image_mb.to_string()),
            Field::boolean("允许 GIF 动画", config.plugins.memes.allow_gif_animation),
            Field::boolean("自动发送", config.plugins.memes.auto_send_enabled),
            Field::new(
                "自动发送概率",
                config.plugins.memes.auto_send_probability.to_string(),
            ),
            Field::new(
                "自动发送最低置信度",
                config.plugins.memes.auto_send_min_confidence.to_string(),
            ),
        ],
        7 => vec![
            Field::boolean("启用", config.plugins.knowledge_base.enabled),
            Field::new("知识库目录", config.plugins.knowledge_base.data_dir.clone()),
            Field::new(
                "搜索最大结果数",
                config.plugins.knowledge_base.max_search_results.to_string(),
            ),
            Field::new(
                "片段上下文字数",
                config
                    .plugins
                    .knowledge_base
                    .snippet_context_chars
                    .to_string(),
            ),
            Field::new(
                "同窗匹配范围",
                config
                    .plugins
                    .knowledge_base
                    .proximity_window_chars
                    .to_string(),
            ),
            Field::new(
                "读取最大行数",
                config.plugins.knowledge_base.max_read_lines.to_string(),
            ),
            Field::new(
                "最大文件 KB",
                config.plugins.knowledge_base.max_file_size_kb.to_string(),
            ),
            Field::boolean(
                "允许 AI 上传",
                config.plugins.knowledge_base.upload_tool_enabled,
            ),
            Field::boolean(
                "启用 Embedding",
                config.plugins.knowledge_base.embedding_enabled,
            ),
            Field::new(
                "Embedding Provider/模型",
                kb_embedding_provider_value(config),
            )
            .choices_owned(provider_model_choice_values(config, false))
            .empty_choice_label("未配置 Embedding"),
            Field::new(
                "语义块大小",
                config
                    .plugins
                    .knowledge_base
                    .semantic_chunk_chars
                    .to_string(),
            ),
            Field::new(
                "语义块重叠",
                config
                    .plugins
                    .knowledge_base
                    .semantic_chunk_overlap
                    .to_string(),
            ),
            Field::new(
                "语义候选数",
                config.plugins.knowledge_base.semantic_top_k.to_string(),
            ),
            Field::new(
                "语义最低分",
                config.plugins.knowledge_base.semantic_min_score.to_string(),
            ),
            Field::new(
                "关键词强命中阈值",
                config
                    .plugins
                    .knowledge_base
                    .keyword_strong_score_threshold
                    .to_string(),
            ),
            Field::new(
                "Embedding 超时秒数",
                config
                    .plugins
                    .knowledge_base
                    .embedding_timeout_seconds
                    .to_string(),
            ),
        ],
        8 => vec![Field::boolean("启用", config.plugins.archlinux.enabled)],
        9 => vec![Field::boolean("启用", config.plugins.man.enabled)],
        10 => vec![
            Field::boolean("启用", config.plugins.memory.enabled),
            Field::boolean(
                "上下文弹出缓存",
                config.plugins.memory.evicted_context_enabled,
            ),
            Field::boolean("联想启用", config.plugins.memory.association_enabled),
            Field::boolean("自动日记", config.plugins.memory.auto_diary_enabled),
            Field::boolean("自动知识记忆", config.plugins.memory.auto_fact_enabled),
            Field::new(
                "联想知识条数",
                config.plugins.memory.association_facts.to_string(),
            ),
            Field::new(
                "联想事件条数",
                config.plugins.memory.association_episodes.to_string(),
            ),
            Field::new(
                "联想字符上限",
                config.plugins.memory.association_max_chars.to_string(),
            ),
            Field::boolean("遗忘启用", config.plugins.memory.forgetting_enabled),
            Field::new(
                "遗忘半衰期天",
                config.plugins.memory.forgetting_half_life_days.to_string(),
            ),
            Field::new(
                "遗忘最低强度",
                config.plugins.memory.forgetting_min_strength.to_string(),
            ),
            Field::new(
                "回忆增强强度",
                config.plugins.memory.forgetting_review_boost.to_string(),
            ),
        ],
        11 => vec![Field::boolean(
            "启用",
            config.plugins.package_advisor.enabled,
        )],
        12 => vec![
            Field::boolean("启用", config.plugins.linux_game_compatibility.enabled),
            Field::new(
                "子代理最大工具次数",
                config
                    .plugins
                    .linux_game_compatibility
                    .max_tool_steps
                    .to_string(),
            ),
        ],
        _ => vec![Field::boolean("启用", plugin_enabled(config, index))],
    }
}

fn apply_plugin_fields(config: &mut AppConfig, index: usize, fields: &[Field]) -> Result<()> {
    match index {
        0 => {
            config.plugins.web.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.web.tavily_api_keys = parse_key_list(&fields[1].value);
            config.plugins.web.firecrawl_api_keys = parse_key_list(&fields[2].value);
            config.plugins.web.anysearch_api_keys = parse_key_list(&fields[3].value);
            config.plugins.web.searxng_base_url =
                fields[4].value.trim().trim_end_matches('/').to_string();
        }
        1 => {
            config.plugins.deep_research.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.deep_research.output_dir = fields[1].value.trim().to_string();
            config.plugins.deep_research.thinking_depth = fields[2].value.trim().to_string();
            config.plugins.deep_research.max_review_revisions = fields[3].value.trim().parse()?;
            config.plugins.deep_research.max_tool_steps_per_round =
                fields[4].value.trim().parse()?;
            config.plugins.deep_research.max_final_answer_chars = fields[5].value.trim().parse()?;
            config.plugins.deep_research.tool_call_timeout_seconds =
                fields[6].value.trim().parse()?;
            config.plugins.deep_research.show_progress = parse_bool_field(&fields[7].value)?;
        }
        2 => {
            config.plugins.vision.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.vision.prefer_current_multimodal_model =
                parse_bool_field(&fields[1].value)?;
            let (provider_id, model) = parse_provider_model_choice(&fields[2].value);
            config.plugins.vision.vision_provider_id = provider_id;
            config.plugins.vision.vision_model = model;
        }
        3 => {
            config.plugins.image_generation.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.image_generation.provider_type = fields[1].value.trim().to_string();
            config.plugins.image_generation.base_url =
                fields[2].value.trim().trim_end_matches('/').to_string();
            config.plugins.image_generation.api_keys = parse_key_list(&fields[3].value);
            config.plugins.image_generation.model = fields[4].value.trim().to_string();
            config.plugins.image_generation.default_aspect_ratio =
                fields[5].value.trim().to_string();
            config.plugins.image_generation.default_resolution = fields[6].value.trim().to_string();
            config.plugins.image_generation.output_dir = fields[7].value.trim().to_string();
            config.plugins.image_generation.auto_print = parse_bool_field(&fields[8].value)?;
            config.plugins.image_generation.timeout_seconds = fields[9].value.trim().parse()?;
        }
        4 => {
            config.plugins.web_images.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.web_images.vision_screening_enabled =
                parse_bool_field(&fields[1].value)?;
            config.plugins.web_images.max_results =
                fields[2].value.trim().parse::<usize>()?.clamp(1, 10);
            config.plugins.web_images.safe_search = parse_bool_field(&fields[3].value)?;
            config.plugins.web_images.auto_preview = parse_bool_field(&fields[4].value)?;
            config.plugins.web_images.preview_count =
                fields[5].value.trim().parse::<usize>()?.min(5);
            config.plugins.web_images.max_download_mb =
                fields[6].value.trim().parse::<f64>()?.clamp(0.1, 50.0);
            config.plugins.web_images.timeout_seconds =
                fields[7].value.trim().parse::<u64>()?.clamp(5, 120);
        }
        5 => {
            config.plugins.print_image.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.print_image.width_percent = fields[1].value.trim().parse::<u8>()?;
            config.plugins.print_image.height_percent = fields[2].value.trim().parse::<u8>()?;
        }
        6 => {
            config.plugins.memes.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.memes.width_percent =
                fields[1].value.trim().parse::<u8>()?.clamp(1, 100);
            config.plugins.memes.height_percent =
                fields[2].value.trim().parse::<u8>()?.clamp(1, 100);
            config.plugins.memes.max_image_mb =
                fields[3].value.trim().parse::<u64>()?.clamp(1, 100);
            config.plugins.memes.allow_gif_animation = parse_bool_field(&fields[4].value)?;
            config.plugins.memes.auto_send_enabled = parse_bool_field(&fields[5].value)?;
            config.plugins.memes.auto_send_probability =
                fields[6].value.trim().parse::<f32>()?.clamp(0.0, 1.0);
            config.plugins.memes.auto_send_min_confidence =
                fields[7].value.trim().parse::<f32>()?.clamp(0.0, 1.0);
        }
        7 => {
            config.plugins.knowledge_base.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.knowledge_base.data_dir = fields[1].value.trim().to_string();
            config.plugins.knowledge_base.max_search_results = fields[2].value.trim().parse()?;
            config.plugins.knowledge_base.snippet_context_chars = fields[3].value.trim().parse()?;
            config.plugins.knowledge_base.proximity_window_chars =
                fields[4].value.trim().parse()?;
            config.plugins.knowledge_base.max_read_lines = fields[5].value.trim().parse()?;
            config.plugins.knowledge_base.max_file_size_kb = fields[6].value.trim().parse()?;
            config.plugins.knowledge_base.upload_tool_enabled = parse_bool_field(&fields[7].value)?;
            config.plugins.knowledge_base.embedding_enabled = parse_bool_field(&fields[8].value)?;
            let (provider_id, model) = parse_provider_model_choice(&fields[9].value);
            config.plugins.knowledge_base.embedding_provider_id = provider_id;
            config.plugins.knowledge_base.embedding_model = model;
            config.plugins.knowledge_base.semantic_chunk_chars = fields[10].value.trim().parse()?;
            config.plugins.knowledge_base.semantic_chunk_overlap =
                fields[11].value.trim().parse()?;
            config.plugins.knowledge_base.semantic_top_k = fields[12].value.trim().parse()?;
            config.plugins.knowledge_base.semantic_min_score = fields[13].value.trim().parse()?;
            config.plugins.knowledge_base.keyword_strong_score_threshold =
                fields[14].value.trim().parse()?;
            config.plugins.knowledge_base.embedding_timeout_seconds =
                fields[15].value.trim().parse()?;
        }
        8 => {
            config.plugins.archlinux.enabled = parse_bool_field(&fields[0].value)?;
        }
        9 => {
            config.plugins.man.enabled = parse_bool_field(&fields[0].value)?;
        }
        10 => {
            config.plugins.memory.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.memory.evicted_context_enabled = parse_bool_field(&fields[1].value)?;
            config.plugins.memory.association_enabled = parse_bool_field(&fields[2].value)?;
            config.plugins.memory.auto_diary_enabled = parse_bool_field(&fields[3].value)?;
            config.plugins.memory.auto_fact_enabled = parse_bool_field(&fields[4].value)?;
            config.plugins.memory.auto_skill_enabled = false;
            config.plugins.memory.association_facts = fields[5].value.trim().parse::<usize>()?;
            config.plugins.memory.association_episodes = fields[6].value.trim().parse::<usize>()?;
            config.plugins.memory.association_max_chars =
                fields[7].value.trim().parse::<usize>()?;
            config.plugins.memory.forgetting_enabled = parse_bool_field(&fields[8].value)?;
            config.plugins.memory.forgetting_half_life_days =
                fields[9].value.trim().parse::<f64>()?;
            config.plugins.memory.forgetting_min_strength =
                fields[10].value.trim().parse::<f64>()?;
            config.plugins.memory.forgetting_review_boost =
                fields[11].value.trim().parse::<f64>()?;
        }
        11 => {
            config.plugins.package_advisor.enabled = parse_bool_field(&fields[0].value)?;
        }
        12 => {
            config.plugins.linux_game_compatibility.enabled = parse_bool_field(&fields[0].value)?;
            config.plugins.linux_game_compatibility.max_tool_steps =
                fields[1].value.trim().parse::<usize>()?.clamp(1, 500);
        }
        _ => {
            let value = parse_bool_field(&fields[0].value)?;
            if plugin_enabled(config, index) != value {
                toggle_plugin(config, index);
            }
        }
    }
    Ok(())
}

fn parse_key_list(value: &str) -> Vec<String> {
    value
        .split(|ch| ch == ',' || ch == '\n' || ch == '\r')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}
