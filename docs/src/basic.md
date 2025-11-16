# Basic Configuration

A good basic configuration would look something like this:

```toml
mam_id = "set mam_id here"

[[qbittorrent]]
url = "http://localhost:8011" # update this to your qbit web ui URL
username = "qbittorent username"
password = "qbittorent password"

[qbittorrent.on_cleaned]
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

### MaM ID
The `mam_id` is a security session you create on <https://www.myanonamouse.net/preferences/index.php?view=security>
