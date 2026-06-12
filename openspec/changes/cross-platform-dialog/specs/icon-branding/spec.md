# icon-branding

Icon resolution and branding elements: main icon, overlay icon, banner, and built-in status icons, with per-platform resolution strategies.

## ADDED Requirements

### Requirement: Icon source resolution
`--icon <value>` SHALL accept: an absolute or relative image file path (PNG/JPG/ICNS/ICO/SVG), an `https://` URL (downloaded at launch), `SF=<symbol.name>` system-symbol syntax, an application path (`.app` on macOS, `.exe`/`.lnk` on Windows — extracting the application's icon), the keywords `default`, `none`, `warning`, `info`, `caution`, and `--iconsize` to control its rendered size (default 150). `--icon none` SHALL hide the icon area and reflow content.

#### Scenario: Icon from file path
- **WHEN** invoked with `--icon /tmp/logo.png`
- **THEN** the image renders in the icon area at the default size of 150 points

#### Scenario: Icon from URL
- **WHEN** invoked with `--icon https://example.com/logo.png` and the URL is reachable
- **THEN** the image is downloaded and displayed

#### Scenario: Unreachable icon URL degrades
- **WHEN** invoked with `--icon https://example.com/missing.png` and the download fails
- **THEN** the default icon is shown, a warning is logged, and the dialog still renders

#### Scenario: Application icon extraction on Windows
- **WHEN** invoked on Windows with `--icon "C:\Program Files\App\app.exe"`
- **THEN** the executable's embedded icon is extracted and displayed

### Requirement: System symbol mapping
`SF=<name>` SHALL render the named SF Symbol on macOS. On Windows, the name SHALL be resolved through a built-in SF-to-Segoe-Fluent-Icons mapping table; if unmapped, a user-supplied icon override pack (directory of `<symbol.name>.png|svg` files, configured via environment variable or config file) SHALL be consulted; if still unresolved, a generic placeholder glyph SHALL be shown with a stderr warning. SF colour modifiers (`colour=`/`color=`, `palette=`) SHALL apply where the resolved glyph supports tinting.

#### Scenario: Mapped SF symbol on Windows
- **WHEN** invoked on Windows with `--icon SF=gear`
- **THEN** the mapped Fluent settings-gear glyph renders in the icon area

#### Scenario: Unmapped SF symbol falls back
- **WHEN** invoked on Windows with `--icon SF=obscure.symbol.name` and no override pack entry exists
- **THEN** a placeholder glyph is shown and a warning naming the symbol is written to stderr

### Requirement: Overlay icon
`--overlayicon <value>` SHALL render a secondary badge icon over the lower-trailing corner of the main icon, accepting the same source syntax as `--icon`.

#### Scenario: Overlay rendered
- **WHEN** invoked with `--icon /tmp/app.png --overlayicon SF=checkmark.circle.fill`
- **THEN** the checkmark glyph renders as a badge over the main icon's corner

### Requirement: Built-in status icons
The keywords `warningicon`, `cautionicon`, and `infoicon` (as flags) and `--icon warning|caution|info` SHALL render platform-appropriate warning/caution/info imagery.

#### Scenario: Warning icon flag
- **WHEN** invoked with `--warningicon`
- **THEN** a warning symbol renders in the icon area

### Requirement: Banner image and banner title
`--bannerimage <path|url>` SHALL render an image across the full top edge of the dialog (default height per upstream, overridable via `--bannerheight`). `--bannertitle` (or `--bannertext`) SHALL overlay the title text on the banner.

#### Scenario: Banner with title
- **WHEN** invoked with `--bannerimage /tmp/banner.png --bannertitle "Welcome"`
- **THEN** the banner image spans the top of the dialog with "Welcome" overlaid as the title
