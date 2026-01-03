//! 配置向导渲染

use crate::tui::components::{render_hint, render_title_block, three_panel_layout, wrap_lines};
use crate::tui::labels::{
    bool_label, classification_label, file_operation_label, month_format_label,
    processing_mode_label,
};
use crate::tui::state::{AppState, ConfigStep, ConfigWizardState};
use crate::tui::theme::{
    config::HIGHLIGHT_SYMBOL,
    theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    widgets::{Block, BorderType, Cell, List, ListItem, Paragraph, Row, Table},
};
use rust_i18n::t;
use std::borrow::Cow;
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;

/// 渲染配置向导
pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = three_panel_layout(area);

    render_title_block(&state.config_wizard.step.title(), frame, header);

    match &state.config_wizard.step {
        ConfigStep::ConfigSelect => draw_config_selection_step(frame, body, state),
        ConfigStep::ConfigForm => draw_config_form(frame, body, state),
        ConfigStep::Summary => draw_summary_step(frame, body, &state.config_wizard),
        ConfigStep::ConfirmRun => draw_confirm_run_step(frame, body, state),
        _ => {}
    }

    let hint_text = match state.config_wizard.step {
        ConfigStep::ConfigForm if state.config_wizard.is_in_input_mode() => t!("input_mode_hint"),
        ConfigStep::ConfigForm => get_form_hint(&state.config_wizard),
        ConfigStep::ConfigSelect => t!("select_config_hint"),
        ConfigStep::Summary => {
            if state.config_wizard.skip_confirm_run {
                t!("config_summary_hint")
            } else if state.config_wizard.is_select_config_flow() {
                t!("config_summary_modify_hint")
            } else {
                t!("config_summary_save_hint")
            }
        }
        ConfigStep::ConfirmRun => {
            if state.config_wizard.is_select_config_flow() {
                t!("confirm_modify_hint")
            } else {
                t!("confirm_run_hint")
            }
        }
        _ => t!("nav_hint"),
    };

    render_hint(&hint_text, frame, footer);
}

fn get_form_hint(state: &ConfigWizardState) -> Cow<'static, str> {
    if state.is_next_selected() {
        return t!("form_next_step_hint");
    }

    let visible_fields = state.get_visible_fields();
    let selected_idx = state.form_state.selected_field;

    if let Some(field) = visible_fields.get(selected_idx).copied() {
        if field.is_input_field() {
            t!("form_string_hint")
        } else {
            t!("form_option_hint")
        }
    } else {
        t!("form_string_hint")
    }
}

fn draw_config_selection_step(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let configs = &state.config_wizard.available_configs;
    let selected = state.config_wizard.selected_value();
    let content_width = list_content_width(area);

    let items: Vec<ListItem> = if configs.is_empty() {
        let message = t!("no_configs_found");
        vec![ListItem::new(wrap_lines(message.as_ref(), content_width)).style(theme().hint())]
    } else {
        configs
            .iter()
            .enumerate()
            .map(|(i, config_path)| {
                let config_name = config_path
                    .file_stem()
                    .map(|os| os.to_string_lossy().to_string())
                    .unwrap_or_else(|| config_path.display().to_string());
                let style = if i == selected {
                    theme().selected()
                } else {
                    theme().normal()
                };
                ListItem::new(wrap_lines(&config_name, content_width)).style(style)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(t!("available_configurations"))
                .border_type(BorderType::Rounded),
        )
        .highlight_style(theme().selected())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

    state.select_state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state.select_state);
}

fn draw_config_form(frame: &mut Frame, area: Rect, state: &mut AppState) {
    draw_form_fields(frame, area, state);
}

fn draw_form_fields(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [title_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);

    let title = Paragraph::new(t!("form_fields_title"))
        .style(theme().title())
        .alignment(Alignment::Left);
    frame.render_widget(title, title_area);

    let visible_fields = state.config_wizard.get_visible_fields();
    let in_input_mode = state.config_wizard.is_in_input_mode();
    let selected_idx = state.config_wizard.form_state.selected_field;
    let content_width = list_content_width(list_area);

    let total_items = visible_fields.len() + 1;

    let safe_selected_idx = if selected_idx >= total_items {
        total_items.saturating_sub(1)
    } else {
        selected_idx
    };

    let items: Vec<ListItem> = visible_fields
        .iter()
        .enumerate()
        .map(|(idx, &field)| {
            let is_selected = idx == safe_selected_idx;
            let value = field.get_value_string(&state.config_wizard);
            let label = field.label();
            let content = if is_selected && in_input_mode && field.is_input_field() {
                let input_buffer = state.config_wizard.input_buffer();
                let cursor = state.config_wizard.input_cursor().min(input_buffer.len());
                let (left, right) = input_buffer.split_at(cursor);
                format!("{}: [{}|{}]", label, left, right)
            } else {
                format!("{}: {}", label, value)
            };
            let content_lines = wrap_lines(&content, content_width);

            let style = if is_selected {
                if in_input_mode && field.is_input_field() {
                    theme().selected().add_modifier(Modifier::ITALIC)
                } else {
                    theme().selected()
                }
            } else {
                theme().normal()
            };

            ListItem::new(content_lines).style(style)
        })
        .chain(std::iter::once({
            let is_selected = safe_selected_idx == visible_fields.len();
            let content = format!("→ {}", t!("go_to_summary"));
            let content_lines = wrap_lines(&content, content_width);
            let style = if is_selected {
                theme().selected().add_modifier(Modifier::BOLD)
            } else {
                theme().normal()
            };
            ListItem::new(content_lines).style(style)
        }))
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(t!("form_fields"))
                .border_type(BorderType::Rounded),
        )
        .highlight_style(theme().selected())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

    state.select_state.select(Some(safe_selected_idx));
    frame.render_stateful_widget(list, list_area, &mut state.select_state);
}

fn draw_confirm_run_step(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let selected = state.config_wizard.selected_value();

    let options = vec![t!("option_yes").to_string(), t!("option_no").to_string()];

    let items: Vec<ListItem> = options
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if i == selected {
                theme().selected()
            } else {
                theme().normal()
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if state.config_wizard.is_select_config_flow() {
        t!("confirm_modify_title")
    } else {
        t!("confirm_run_title")
    };

    let list = List::new(items)
        .block(
            Block::bordered().title(title).border_type(BorderType::Rounded),
        )
        .highlight_style(theme().selected())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

    state.select_state.select(Some(selected));
    frame.render_stateful_widget(list, area, &mut state.select_state);
}

fn draw_summary_step(frame: &mut Frame, area: Rect, wizard: &ConfigWizardState) {
    let config = wizard.build_config();
    let value_width = summary_value_width(area);

    let rows = vec![
        Row::new(vec![
            Cell::from(t!("summary_input")),
            Cell::from(wrap_lines(&format_paths(&config.input_dirs), value_width)),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_output")),
            Cell::from(wrap_lines(
                &config.output_dir.display().to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_mode")),
            Cell::from(wrap_lines(
                &processing_mode_label(config.processing_mode).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_classify")),
            Cell::from(wrap_lines(
                &classification_label(config.classification).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_month_format")),
            Cell::from(wrap_lines(
                &month_format_label(config.month_format).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_operation")),
            Cell::from(wrap_lines(
                &file_operation_label(config.operation).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_deduplicate")),
            Cell::from(wrap_lines(
                &bool_label(config.deduplicate).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_dry_run")),
            Cell::from(wrap_lines(
                &bool_label(config.dry_run).to_string(),
                value_width,
            )),
        ]),
        Row::new(vec![
            Cell::from(t!("summary_classify_by_type")),
            Cell::from(wrap_lines(
                &bool_label(config.classify_by_type).to_string(),
                value_width,
            )),
        ]),
    ];

    let table = Table::new(rows, [Constraint::Length(15), Constraint::Fill(1)])
        .block(
            Block::bordered()
                .title(t!("configuration_summary"))
                .border_type(BorderType::Rounded),
        )
        .column_spacing(2)
        .style(theme().normal());

    frame.render_widget(table, area);
}

fn list_content_width(area: Rect) -> usize {
    let inner_width = area.width.saturating_sub(2) as usize;
    let highlight_width = UnicodeWidthStr::width(HIGHLIGHT_SYMBOL);
    inner_width.saturating_sub(highlight_width).max(1)
}

fn summary_value_width(area: Rect) -> usize {
    let inner_width = area.width.saturating_sub(2) as usize;
    let label_width = 15usize;
    let column_spacing = 2usize;
    inner_width
        .saturating_sub(label_width + column_spacing)
        .max(1)
}

fn format_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        String::new()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    }
}
