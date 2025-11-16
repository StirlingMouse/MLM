# Search

MLM has a very basic torrent search interface for manually searching and downloading torrents. This will over time be extended to support all search options.
For now the only configuration option for the search is:

```toml
[search]
wedge_over = "30 MiB"
```

### Wedge Over
A size of torrents which over that will be automatically wedged when you select them for download in the WebUI (shown by the download arrow turning blue)
