use crossterm::event;

pub enum Signal {
    TermEvent(event::Event)
}

pub enum Action {
    Draw
}