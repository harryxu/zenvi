use gpui::actions;

actions!(
    zenvi,
    [
        Quit,
        NewWindow,
        OpenFile,
        OpenFolder,
        OpenConfig,
        CloseBuffer,
        Escape,
        ReloadNvim,
        InstallCli,
    ]
);
