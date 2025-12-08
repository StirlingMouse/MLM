# Autograbbers

Autograbbers are predefined searches that are run periodically. Very similar to RSS.
MLM monitors both your ratio and your unsat slots to try and avoid causing any problems for your account.
By default MLM will never download a torrent that will take you below 2 in ratio, and always leave 10 unsats slots free.

A basic configuration for an autograbber look like this:
```toml
[[autograb]]
type = "bookmarks"
cost = "free"
```

Most of the fields you can set on an autograb block is shared by other blocks that select torrents, see them in [Search Filters](./search_filters.md).

### Type
`type` selects which torrents to search, the different options are:

```toml
type = "bookmarks"        # Searches your bookmarks
type = "freeleech"        # Searches the freeleech list
type = "new"              # Searches new/latest torrents
type = { uploader = 123 } # Searches uploads by user with id 123
```

Bookmarks and Freeleech will go through up to 50 pages of the search result,
allowing you to find all of them. While New and Uploader will only ever
see the 100 latest torrents matching your [search filters](./search_filters.md).

### Cost
`cost` selects which kind of torrents to grab. This is by default `free` but the different options are:

```toml
cost = "free"      # Free for you in any way, VIP, Personal Freeleeech or Global Freeleech
cost = "wedge"     # Apply a freeleech wedge before downloading
cost = "try_wedge" # Try to apply a freeleech wedge before downloading, but still download if that is not possible
cost = "ratio"     # Download the torrent even if you will take a ratio hit
```

### Query and Search In
A search query, same as the search field on MaM, example:
```toml
query = '"Akwaeke Emezi"|"T J Klune"'
search_in = [ "author" ] # only search in authors
```

The `search_in` control what the query should match, here it only matchers author names but it can be one or more of:

 - author
 - description
 - filenames
 - filetypes
 - narrator
 - series
 - tags
 - title

The MaM search is very powerful, checkout the on-site search quide for what you can do with it: <https://www.myanonamouse.net/guides/?gid=37729>

### Name
```toml
name = "My Bookmarks"
```
Name this autograbber, this allows you to see which autograbber is responsible for downloading a torrent in the event log.

### Sort By
Normally results are sorted by newest first so that the New and Uploader types gets the latest torrents. The options are:

```toml
sort_by = "oldest_first" # Find oldest matching torrents instead of the newest ones
sort_by = "random"       # Randomize order
sort_by = "low_seeders"  # Find lowest seeded torrents
sort_by = "low_snatches" # Find lowest snatched torrents
```

Random is interesting as it can be used to effectively fill your unsat slots with _something_, example to grab random LGBT torrents if you have more than 50 unsat slots free:
```toml
[[autograb]]
name = "LGBT"
type = "new"
query = 'm4b|epub'
search_in = [ "filtetypes" ]
sort_by = "random"
flags = { lgbt = true }
unsat_buffer = 50
```

### Max Pages
```toml
max_pages = 5
```
How many pages (of 100 torrents) of the search should be fetched. By default, type Bookmarks and Freeleech will go through up to 50 pages of the search result, while other types only fetch a single page.

### Search Interval
```toml
search_interval = 60
```
In minutes, how often this search should be done

### Unsat Buffer
```toml
unsat_buffer = 10
```
How many unsat slots that should be left open so that you have room to download torrents manually or with other autograb blocks.

### Wedge Buffer
```toml
wedge_buffer = 10
```
How many wedges that should be left unused so that you have can download torrents manually or with other autograb blocks.

### Max Active Downloads
```toml
name = "My Bookmarks"
max_active_downloads = 10
```
How many currently active downloads are allowed for this autograb block. For example useful if you grab low-seeded torrents. Requires that you also set a name so the active torrents can be identified.

### Category
```toml
category = "bookmarks"
```
A qBittorrent category to set on all torrents downloaded by this autograbber. Overrides any `[[tag]]` blocks you might have.

### Dry Run
```toml
dry_run = true
```
Prevents the autograbber from actually downloading anything. You can use this to look at the logfiles/docker logs for the searches, or use the `search on MaM` links on the config page, to help figure out if you are matching the torrents that you want.
