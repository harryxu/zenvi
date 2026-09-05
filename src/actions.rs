use gpui::actions;

actions!(
    zenvi,
    [
        Quit,
        About,
        NewWindow,
        OpenFile,
        OpenFolder,
        OpenConfig,
        CloseBuffer,
        Escape,
        ReloadNvim,
        InstallCli,
        Paste,
        Copy,
        Cut,
        SelectAll,
        Undo,
        Redo,
        ToggleLeftPanel,
    ]
);
