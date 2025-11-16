# Library Organizer

The library organizer hardlinks files into a directory with the directory structure:
```sh
"Author/Series/Series #1 - Title {Narrator}/" # For audiobooks with a series
"Author/Title {Narrator}/" # For audiobooks without a series
"Author/Series/Series #1 - Title/" # For ebooks with a series
"Author/Title/" # For ebooks without a series
```

If you do not want to include narrator in the audiobook folder names, the top level option
```toml
exclude_narrator_in_library_dir = true
```
can be set. This makes the MLM directory structure match booktree which allows easier migration from booktree.

You can select either a category or a qBittorrent download directory to link to a library.

Link all torrents with category "Audiobooks" to "/mnt/Data/Library/Audiobooks":
```toml
[[library]]
category = "Audiobooks"
library_dir = "/mnt/Data/Library/Audiobooks"
```

Link all torrents with download directory/save location "/mnt/Data/Downloads/Ebooks" to "/mnt/Data/Library/Audiobooks":
```toml
[[library]]
download_dir = "/mnt/Data/Downloads/Ebooks"
library_dir = "/mnt/Data/Library/Ebooks"
```

<div class="warning">
Note that you can use either `category` or `download_dir` to select torrents for a library, not both.
</div>

It's possible to use tags to additionally filter down which torrents to link:
```toml
[[library]]
download_dir = "/mnt/Data/Uploads/Audiobooks"
library_dir = "/mnt/Data/Library/Audiobooks"
allow_tags = [ "library" ] # only links torrents with tag "library"
deny_tags = [ "skip" ] # but not if also having tag "skip"
```

When specifying multiple `allow_tags`, the torrent just need to have any of them to be linked.
When specifying multiple `deny_tags`, the torrent just need to have any of them to be skipped.

### Method
It's possible to instead copy or symlink files to the library if hardlinking does not work for you:
```
method = "hardlink_or_copy" # Try hardlinking but fallback to copy if required
method = "hardlink_or_symlink" # Try hardlinking but fallback to symlink if required
method = "copy" # Always copy files
method = "symlink" # Always symlink files
```

Example usage:
```toml
[[library]]
category = "Audiobooks"
library_dir = "/mnt/Data/Library/Audiobooks"
method = "copy"
```

### File Types
A list of audio and ebook file types in order of preference that will be linked from this library, the default config are:
```toml
audio_types = ["m4b", "m4a", "mp4", "mp3", "ogg"]
ebook_types = ["cbz", "epub", "pdf", "mobi", "azw3", "azw", "cbr"]
```

Only one format from each list will be linked. This means that a multi-format ebook torrent will only have its best format linked. E.g. for a torrent with `epub`, `pdf` and `mobi` files, only the `epub` will be linked. But as one format from each list is selected, an audiobook torrent with a supplementary PDF will have both the audiofiles and the PDF linked.
