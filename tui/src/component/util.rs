use std::usize;

use crossterm::event::MouseEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::Style,
    text::Line,
};

use super::Component;

#[derive(Debug, Clone)]
pub struct CenteredBox {
    pub centered: Rect,
    pub center: [Rect; 2],
    pub left: Rect,
    pub right: Rect,
}

pub fn centered_box_sized(rect: Rect, width: u16, height: u16) -> CenteredBox {
    let x_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(rect);
    let y_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(x_layout[1]);
    CenteredBox {
        centered: y_layout[1],
        center: [y_layout[0], y_layout[2]],
        left: x_layout[0],
        right: x_layout[2],
    }
}

pub fn centered_box_percentage(rect: Rect, x_percent: u16, y_percent: u16) -> CenteredBox {
    let x_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - x_percent) / 2),
            Constraint::Percentage(x_percent),
            Constraint::Percentage((100 - x_percent) / 2),
        ])
        .split(rect);
    let y_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - y_percent) / 2),
            Constraint::Percentage(x_percent),
            Constraint::Percentage((100 - y_percent) / 2),
        ])
        .split(x_layout[1]);
    CenteredBox {
        centered: y_layout[1],
        center: [y_layout[0], y_layout[2]],
        left: x_layout[0],
        right: x_layout[2],
    }
}

pub fn mouse_contains_option<'a, T: Component>(
    mouse: &MouseEvent,
    component: Option<&'a mut T>,
) -> (bool, &'a mut T) {
    let com = component.unwrap();
    (
        com.mouse_area()
            .contains(Position::new(mouse.column, mouse.row)),
        com,
    )
}

pub fn horizontal_centered(length: u16, rect: Rect) -> Rect {
    Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(length),
        Constraint::Min(0),
    ])
    .split(rect)[1]
}

pub fn mouse_contains(mouse: &MouseEvent, com: &impl Component) -> bool {
    com.mouse_area()
        .contains(Position::new(mouse.column, mouse.row))
}

pub fn str_to_lines(str: &str, width: u16, style: Style, vec: &mut Vec<Line>) {
    let chars = str.chars().collect::<Vec<char>>();
    let length = chars.len();
    let width = width as usize;
    let mut curr = 0;
    while curr < length {
        let end = if curr + width > length {
            length
        } else {
            curr + width
        };
        vec.push(Line::from(chars[curr..end].iter().collect::<String>()).style(style));
        curr = end;
    }
}
