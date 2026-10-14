# Boot fixtures (T1.2)

`mock-boot-up.webm` (2 s) and `mock-boot-down.webm` (1.5 s): original test clips made with ffmpeg
(a coloured frame and a sine tone), standing in for the user's custom Boot Up / Boot Down videos in
the mocked UI tests. WebM (VP9 + Opus) because the open-source Chromium used by Playwright has no
H.264 / AAC; WebView2 on the PC plays the user's MP4 files. The user's own videos are never committed.
