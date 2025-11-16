# Tagging

`[[tag]]` blocks can be used to set a category or tags on torrents in qBittorrent.

Fields for selecting torrents are the same as for other blocks, see them in [Search Filters](./search_filters.md).

### Category
```toml
categories = { audio = false, ebook = true }
category = "Ebooks"
```
Sets the qBittorrent category of a torrent, in this example of all ebook torrents.
If multiple `[[tag]]` blocks matches a torrent, only the first category is used.

### Tags
```toml
[[tag]]
flags = { explicit = true }
tags = [ "explicit" ] # However tags from different tag section merge so a torrent can get both explicit, abridged and one of the categories
```
Sets qBittorrent tags on a torrent, in this example sets the tag `explicit` on all torrents that have the explicit flag.
If multiple `[[tag]]` blocks matches a torrent, tags from all of them are set.
