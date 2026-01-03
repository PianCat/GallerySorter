//! 主菜单渲染

use crate::tui::components::{render_hint, render_title_block, three_panel_layout};
use crate::tui::state::{AppState, MenuItem};
use crate::tui::theme::{config::HIGHLIGHT_SYMBOL, theme};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, BorderType, List, ListItem},
};
use rust_i18n::t;

/// 渲染主菜单
pub fn draw(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let [header, body, footer] = three_panel_layout(area);

    render_title_block(&t!("select_option"), frame, header);

    let items: Vec<ListItem> = MenuItem::iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.menu_state.selected() {
                theme().selected()
            } else {
                theme().normal()
            };
            ListItem::new(item.label()).style(style)
        })
        .collect();

    let menu_list = List::new(items)
        .block(
            Block::bordered()
                .title(t!("select_option"))
                .border_type(BorderType::Rounded)
                .border_style(theme().border()),
        )
        .highlight_style(theme().selected())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

    frame.render_stateful_widget(menu_list, body, &mut state.menu_state.list_state);

    render_hint(&t!("menu_hint"), frame, footer);
}
