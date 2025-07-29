# Docker

The recommended installation method on anything but Windows is to use docker compose.

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

