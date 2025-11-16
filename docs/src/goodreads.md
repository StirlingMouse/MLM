# Goodreads Import

Goodreads lists/bookshelves can be used as a source for autograbbing books.
On the bookshelf page, use the RSS icon: ![](https://s.gr-assets.com/assets/links/rss-d17345b73ab0388f7a23933239a75efb.gif) to get the URL to give to MLM.

Example configuration:
```toml
[[goodreads_list]]
url = "https://www.goodreads.com/review/list_rss/..." # RSS feed of a Goodreads list

[[goodreads_list.grab]] # A block deciding what torrents to grab from the list,
                        # if a torrent matches multiple blocks, the first one is used
cost = "all"
languages = [ "english" ] # you can use the same search filters as for autograb blocks
max_size = "15 MiB"

[[goodreads_list.grab]]
cost = "wedge" # automatically wedge torrents before download, as we have an cost=all block
               # before this one with a max_size, this will only wedge torrents > 15 MiB
languages = [ "english" ]
```

Each list needs at least one `goodreads_list.grab` block that select what torrents to grab. To see how to select torrents and what fields you can set, see [Search Filters](./search_filters.md).
