# Search Filters

The `[[autograb]]`, `[[goodreads_list.grab]]`, and `[[tag]]` blocks in the config take options that map the same search filters you find on MaM.

### Categories
```toml
categories = { audio = false, ebook = [ "food" ] }
```

Selects torrents by category, `audio = false` filters out all audiobook torrents. `audio = true` would select all audiobook torrents. If you only want to select some categories of a main type, give an array of category names which are the same as you find in the search form.

The above example selects Ebook - Food only.

To select only audiobooks:
```toml
categories = { audio = true, ebook = false }
```

To select Fantasy and Romance from either main cat:
```toml
categories = { audio = [ "fantasy", "romance" ], ebook = [ "fantasy", "romance" ] }
```

### Languages
```toml
languages = [ "English" ]
```

An array of languages to allow. Uses the same names as can be found in the search form.

To select both English and French torrents:
```toml
languages = [ "English", "French" ]
```

### Flags
```toml
flags = {
    crude_language = false, # Exclude torrents with the "crude language" flag
    violence = false, # Exclude torrents with the "violence" flag
    some_explicit = false, # Exclude torrents with the "some explicit" flag
    explicit = true, # Only select torrents with the "explicit" flag
    abridged = true, # Only select torrents with the "abridged" flag
    lgbt = true, # Only select torrents with the "lgbt" flag
}
```

Flags are set to `true` or `false` to either require or exclude them. Skip a flag if you do not care about if it's set or not. Unlike the normal search form you can both exclude and select for at the same time.

To filter out explicit torrents when ignoring other flags:
```toml
flags = { explicit = false }
```

To only select abridged torrents when ignoring other flags:
```toml
flags = { abridged = true }
```

### Exclude uploader
```toml
exclude_uploader = [ "username" ]
```
A list of uploader usernames to filter out, useful if you don't want to download your own uploads.

### Size
```toml
min_size = "100 KiB"
max_size = "1.2 GiB"
```
Only select torrents above/below the specified size

### Upload date
```toml
uploaded_after = "2020-06-01"
uploaded_before = "2025-01-01"
```
Only select torrents uploaded before/after the specified date. Inclusive so this will also select torrents uploaded on 2020-06-01 and 2025-01-01.

### Peers
```toml
min_seeders = 10
max_seeders = 50
min_leechers = 10
max_leechers = 50
min_snatched = 10
max_snatched = 50
```

Only select torrents with seeders/leechers/snatches above or below the specified value. Inclusive so this also selects torrents with 10 or 50 seeders.
