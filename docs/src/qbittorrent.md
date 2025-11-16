# qBittorrent

One or more `[[qbittorrent]` blocks has to be configured. If multiple qBittorrent blocks are configured, torrents from all qBittorrent instances can be linked, but newly downloaded torrents will only be added to the first configured instance.

Basic configuration just requires an URL:
```toml
[[qbittorrent]]
url = "http://localhost:8011"
```

If your qBittorrent instance requires authentication, set it with:
```toml
username = "qbittorent username"
password = "qbittorent password"
```

### Path Mapping
If your qBittorrent instance is set up so that it uses different paths to refer to a file than MLM, you'll need to configure path mapping.

 For Example, if qBittorrent refers to a file with `/downloads/My Audiobook.m4b` but MLM will find it under `/mnt/data/My Audiobook.m4b` you need the setting:
```toml
path_mapping = { "/downloads" =  "/mnt/data" }
```

### On Cleaned
If you download a better version of a torrent (e.g. an `m4b` torrent when you previously had an `mp3`), the older torrent will be "cleaned". By default this only means having its hardlinks removed. However you can also change the qBittorrent category and/or set tags.
```toml
[qbittorrent.on_cleaned]
category = "Seed"
tags = [ "superseded" ]
```
