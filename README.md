# MLM - Myanonamouse Library Manager

MLM is both an auto downloader and a library organizer. Both parts are optional so either can be replaced with e.g. RSS or [booktree](https://github.com/myxdvz/booktree) if you prefer. And even if you use both, you can still add torrents manually and have them organized, and/or use booktree for collections or files that are not from MaM.

This allows you to automatically download for example bookmarks and have them hardlinked into an organized library folder for e.g. ABS. It also follows a list of preferred formats so that if you first download the mp3 version of a book and then later download an m4b, the mp3 will be automatically removed from your library and optionally moved to a different category or tagged in qBittorrent.

The auto downloader can both use configured searches similar to RSS, and Goodreads lists as input. It keeps track of your unsat slots and will by default always leave at least 10 open. It also keeps track of your library and avoids downloading e.g. an mp3 torrent if you already have the m4b.

The library organizer will only link one audio file type and one ebook file type per torrent. So e.g. an audiobook torrent with both m4b and pdf files will have both linked, but an ebook torrent with both and epub and mobi will only have the epub linked.

Limitations:

- At the moment MLM only works with qbittorrent
- MLM works with torrents, meaning collections (multiple books in a single torrents) will be treated as one book (however if you link these with [booktree](https://github.com/myxdvz/booktree), MLM will not touch those files)
- MLM works with torrents from MaM, meaning files not via a torrent from here can not be handled (however if you link these with [booktree](https://github.com/myxdvz/booktree), MLM will not touch those files)

It is available as both a docker container and a Windows application. See the docs for install and configuration instructions:
https://stirlingmouse.github.io/MLM/introduction.html
