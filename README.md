# clipr

The CLI captures system audio and allows to make clips based on the 
specified buffer length (`duration` flag).

## Usage

```
Usage: clipr [OPTIONS]

Options:
  -d, --duration <DURATION>        [default: 30]
  -d, --date-format <DATE_FORMAT>  [default: %Y%m%d_%H%M%S]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Keybindings

```
Ctrl+Alt/Option+S: Save recording to clipr/recorded_<timestamp>.wav
```

## Default clip directory

- Linux: `/home/alice/Music/clipr`
- Mac: `/Users/Alice/Music/clipr`
- Windows: `C:\Users\Alice\Music\clipr`
