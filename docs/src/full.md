# Full Example

An advanced example showcasing many options:
```toml
mam_id = "set mam_id here"

# Below top-level settings show the default values
web_host = "0.0.0.0" # What address to bind the web server to
web_port = 3157 # What port to bind the web server to
unsat_buffer = 10 # How many unsat slots to leave empty
min_ratio = 2 # Lowest ratio MLM is allowed to use. If downloading a torrent would take you below this ratio, MLM will not download it.
add_torrents_stopped = false
exclude_narrator_in_library_dir = false
search_interval = 30 # in minutes, how often a search should be done for the autograbs
goodreads_interval = 60 # in minutes, how often the goodreads lists should be checked and books searched for
link_interval = 10 # in minutes, how often the library organizer should query qbittorent for new torrents
audio_types = ["m4b", "m4a", "mp4", "mp3", "ogg"] # order of preference for audiobook formats, formats not in this list will not be downloaded or linked
ebook_types = ["cbz", "epub", "pdf", "mobi", "azw3", "azw", "cbr"] # order of preference for ebook formats, formats not in this list will not be downloaded or linked

[[qbittorrent]]
url = "http://localhost:8011"
username = "qbittorent username"
password = "qbittorent password"

[qbittorrent.on_cleaned]
category = "Seed"
tags = [ "superseded" ]

[[qbittorrent]] # you can have multiple qbittorent instances. However autograbbed torrents will only be added to the first one
url = "http://localhost:8012"

[qbittorrent.on_cleaned] # Every qbit instance has their own on_cleaned rules
tags = [ "superseded" ]

[[autograb]]
type = "freeleech" # autograbs from the global freeleech list only, not PF or VIP
languages = [ "english" ] # you can filter by language
flags = { lgbt = true } # or flags, true requires the flag to exist
min_size = "50 MiB"
exclude_uploader = [ "Oriel" ] # and exclude certain uploaders (excluding yourself is a good idea if you use multiple clients!)
unsat_buffer = 50 # you can set a different unsat buffer for a specific autograb if you don't want it to overwhelm your slots

[[autograb]]
type = "new" # autograbs from newly uploaded torrents
query = '"Akwaeke Emezi"|"T J Klune"' # a normal search query, see the search guide for what you can do: https://www.myanonamouse.net/guides/?gid=37729
search_in = [ "author" ] # only search in authors
flags = { violence = false } # with false we forbid the flag
dry_run = true # this makes MLM not actually download the selected torrents, only log them, for testing your search

# Other filters you can use
# uploaded_after = "2020-06-01"
# uploaded_before = "2025-01-01"
# min_seeders = 10
# max_seeders = 10
# min_leechers = 10
# max_leechers = 10
# min_snatched = 10
# max_snatched = 10

[[goodreads_list]]
url = "https://www.goodreads.com/review/list_rss/..." # RSS feed of a Goodreads list

[[goodreads_list.grab]] # A block deciding what torrents to grab from the list, if a torrent matches multiple block the first one is used
cost = "all"
languages = [ "english" ] # you can use the same search filters as for autograb blocks
max_size = "15 MiB"

[[goodreads_list.grab]]
cost = "wedge" # automatically wedge torrents before download, as we have an cost=all block before with a max_size, this will only wedge torrents > 15 MiB
languages = [ "english" ]

[[goodreads_list]]
url = "other list"

[[goodreads_list.grab]] # each list has their own grab blocks to select torrents
cost = "free"
prefer_format = "audio" # If a torrent is available in both ebook and audio, only download the audiobook
                        # however this still allow downloading an ebook if no audiobook is available.
                        # leave this property out to download both formats.
                        # If you never want to download an ebook, use categories = { audio = true, ebook = false } instead

[[tag]]
categories = { audio = false, ebook = [ "food" ] }
category = "Cookbooks" # Cookbooks will win over Ebooks as it is defined first and a torrent can only have one category

[[tag]]
flags = { explicit = true }
tags = [ "explicit" ] # However tags from different tag section merge so a torrent can get both explicit, abridged and one of the categories

[[tag]]
flags = { abridged = true }
tags = [ "abridged" ]

[[tag]] # you can skip the tag sections if you don't use tagging or categories in qbittorrent
categories = { audio = true, ebook = false } # this selects all audiobook torrents
category = "Audiobooks" # and sets the qbittorrent category to "Audiobooks"

[[tag]]
categories = { audio = false, ebook = true }
category = "Ebooks"

[[library]]
category = "Audiobooks"
library_dir = "/mnt/Data/Library/Audiobooks"

[[library]]
download_dir = "/mnt/Data/Downloads/Ebooks" # you can also specify a library using the download_dir
library_dir = "/mnt/Data/Library/Ebooks"

[[library]]
download_dir = "/mnt/Data/Uploads/Audiobooks" # multiple libraries can contribute to the same library dir, for example if you keep your own uploads separate
library_dir = "/mnt/Data/Library/Audiobooks"
allow_tags = [ "library" ] # you can also require the torrents to have certain tags
deny_tags = [ "skip" ] # or disallow some
```
