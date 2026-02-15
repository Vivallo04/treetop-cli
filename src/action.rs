#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // New variants wired in upcoming steps
pub enum Action {
    Quit,
    Navigate(Direction),
    Kill(u32),
    ForceKill(u32),
    EnterFilterMode,
    ExitFilterMode,
    ClearFilter,
    UpdateFilter(String),
    CycleColorMode,
    CycleTheme,
    ToggleDetailPanel,
    ToggleHelp,
    CycleSortMode,
    Refresh,
    ZoomIn,
    ZoomOut,
    SelectAt(u16, u16),
    None,
}
