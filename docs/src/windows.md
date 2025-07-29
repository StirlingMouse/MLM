# Windows

For Windows, download the installer from:
<https://github.com/StirlingMouse/MLM/releases/latest>

After starting MLM a small mouse icon will be placed in your tray icons. From here you can access the Web UI, the config file, and the logs directory.

If using the Windows qBittorrent version, also remember to enable its Web UI under settings.

Configure MLM to connect to qbittorent with a configuration like:

```toml
[[qbittorrent]]
url = "http://localhost:8080"
username = "qbittorent username"
password = "qbittorent password"
```

Make sure the port number (8080) matches the port configured in qBittorrent, as well as the username and password. Or leave those out if "Bypass authentication for clients on localhost" is checked.
