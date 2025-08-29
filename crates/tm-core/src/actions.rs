#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveDown,          // j / Down
    MoveUp,            // k / Up
    HalfPageDown,      // Ctrl-d
    HalfPageUp,        // Ctrl-u
    GoTop,             // gg
    GoBottom,          // G
    FocusFilter,       // /
    Quit,              // q  (TUI only, GUI ignores)

    // Status management
    StatusNext,    // cycle forward
    StatusPrev,    // cycle backward
    SetTodo,       // force todo
    SetDoing,      // force in-progress
    SetDone,       // force done

}

