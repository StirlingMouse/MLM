# Audiobookshelf

MLM creates `metadata.json` files when linking which allows audiobookshelf to get great metadata without further configuration. However, you can configure it to also talk with audiobookshelf via its API for further integration.

```toml
[audiobookshelf]
url = "https://audiobookshelf.my.domain"
token = ""
```

`token` is an API Key that you create in Audiobookshelf > Settings > API Keys > Add API Key.


When this option is set, MLM will add ABS links on all torrents that have been picked up by ABS so that you can easily open them from MLM.

It will also update the metadata in ABS after linking, so if the uploader or torrent mods correct a torrent, that gets reflected in ABS.

Finally, if a torrents gets cleaned without a replacement (can be done manually in the WebUI) or is replaced under a different path, MLM will automatically remove the book from ABS where they otherwise would show up as "issues" with "missing files".
