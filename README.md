# MLM - Myanonamouse Library Manager

NOTE: MLM is very early software, I'd advice that you only try it out at this time if you are happy to read very spammy logs if things doesn't work as expected

MLM combines an auto downloader with a library organizer. This allows you to automatically download for example bookmarks and have them hardlinked into an organized library folder for e.g. ABS. It also follows a list of preferred formats so that if you first download the mp3 version if a book and then later download an m4b, the mp3 will be automatically removed from your library and optionally moved to a different category or tagged in qbittorrent.
The auto downloader keeps track of your unsat slots and will by default always leave at least 10 open. It also keeps track of your library and avoids downloading e.g. an mp3 torrent if you already have an m4b.
The library organizer will only link one audio file type and one ebook file type per torrent. So e.g. an audiobook torrent with both m4b and pdf files will have both linked, but an ebook torrent with both and epub and mobi will only have the epub linked.

The auto downloader and library organizer are both optional parts so either one can be replaced with e.g. RSS or [booktree](https://github.com/myxdvz/booktree) if you prefer. And even if you use both, you can still add torrents manually and have them organized, and/or use booktree for collections or files that are not from MaM.

Limitations:
 - At the moment MLM only works with qbittorrent
 - MLM works with torrents, meaning collections (multiple books in a single torrents) will be treated as one book (however if you link these with [booktree](https://github.com/myxdvz/booktree), MLM will not touch those files)
 - MLM works with torrents from MaM, meaning files not via a torrent from here can not be handled (however if you link these with [booktree](https://github.com/myxdvz/booktree), MLM will not touch those files)
 - Metadata will only ever be as good as it is on MaM. Please submit updated information or contact staff on torrents to help improve metadata for everyone

All it's features are configured using a config file, a good setup to start with would be something like:
```toml
mam_id = "set mam_id here"

[[qbittorrent]]
url = "http://localhost:8011" # update this to your qbit web ui URL
username = "qbittorent username"
password = "qbittorent password"

[qbittorrent.on_cleaned] # you can skip the on_cleaned section if you don't want MLM to do anything in qbit when upgrading a book
category = "Seed" # change the qbittorrent category to Seed when a torrent gets replaced with a better one

[[autograb]] # you can skip the autograb sections if you don't want it to grab anything automatically
type = "bookmarks" # this grabs all your bookmarks that are freeleech (global, personal or VIP)
cost = "free" # cost defaults to free so you never accidentally ruin your ratio if you forget to set it

[[autograb]]
type = "bookmarks" # this grabs all your bookmarks smaller than 15 MiB even if they are not freeleech
cost = "all"
max_size = "15 MiB"

[[tag]] # you can skip the tag sections if you don't use tagging or categories in qbittorrent
categories = { audio = true, ebook = false } # this selects all audiobook torrents
category = "Audiobooks" # and sets the qbittorrent category to "Audiobooks"

[[tag]]
categories = { audio = false, ebook = true }
category = "Ebooks"

[[library]] # you can skip the library sections if you don't want MLM to organize a library folder for you
category = "Audiobooks" # this selects all torrents with the category Audiobooks
library_dir = "/mnt/Data/Library/Audiobooks" # this is where your nicely organized audiobooks will end up

[[library]]
category = "Ebooks"
library_dir = "/mnt/Data/Library/Ebooks"
```

Example docker compose file:
```yaml
services:
  mlm:
    image: ghcr.io/stirlingmouse/mlm:main 
    volumes:
      - ./config:/config # folder for the config file, place it in config/config.toml
      - ./data:/data # folder where mlm will keep a database
      - /mnt/Data:/mnt/Data # folder where your downloaded files and library can be accessed from
    environment:
      TZ: Europe/London # https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
```

A more advanced example showcasing all current options with their default values:
```toml
mam_id = "set mam_id here"
web_host = "0.0.0.0" # What address to bind the web server to
web_port = 3157 # What port to bind the web server to
unsat_buffer = 10 # how many unsat slots to leave empty
add_torrents_stopped = false
search_interval = 30 # in minutes, how often a search should be done for the autograbs
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

I will never promise any future development, but some of my current plans:
- Add more search filters
- Add a web ui to show status and allow a few actions (like updating metadata for a torrent you suggested updated info for)
- Add a plain .torrent file mode that can work with any client
- Add a way to handle requests, e.g. with Goodreads lists or "to read" on The Storygraph
- Support auto applying wedges to non-free torrents
- Support editing the config file running


