# Settings inventory

No settings screen yet (T3). Settings-like behaviour in T0:

| Setting | Where | Default | Notes |
|---|---|---|---|
| Start with Windows | `AIONE4.exe --autostart=on` / `--autostart=off` | off | Writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\AIONE 4`. Becomes a Settings toggle in T3. |
| Copy previous AIONE data | first-start question | — | Asked once; decision in `migration.json`. |
