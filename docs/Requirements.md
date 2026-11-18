# ESP32 Desktop Terminal — Requirements

| | |
|---|---|
| Status | Draft v0.2 |
| Date | 2026-11-17 |
| Sources | `ReferenceDocs/ProjectOverview.md`, `Hardware.md`, `Pins.md`, `APIs.md`, `E-Paper_ESP32_Driver_Board_V3.pdf` (schematic) |
| Related | [Architecture](Architecture.md) · [Implementation Plan](ImplementationPlan.md) |

## 1. Purpose

A desk-top information terminal: an ESP32 driving a 7.5" e-ink panel that shows
weather, news and market data on four pages selected with four physical buttons.

## 2. How to read this document

Every requirement has a stable ID (`REQ-<AREA>-<NN>`) that the architecture and
the implementation plan refer to.

The **Origin** column separates what the reference docs say from what was added
to make them buildable:

- **Stated** — taken directly from `ReferenceDocs/`.
- **Derived** — not written in the reference docs; added because a stated
  requirement cannot be met, tested or shipped without it. Derived requirements
  are proposals and are the ones most worth reviewing.

"Shall" is mandatory for v1. "Should" is a goal that may be traded off.

## 3. Hardware

| ID | Requirement | Origin |
|---|---|---|
| REQ-HW-01 | The firmware shall target the ESP32-WROOM-32E on the Waveshare E-Paper ESP32 Driver Board, Rev 3 (SKU 15823). | Stated; revision from schematic |
| REQ-HW-02 | The firmware shall drive the Waveshare 7.5" e-ink panel, model 075BN-T7-D2. This is the 800×480, 1-bit black/white panel. | Stated; variant confirmed by owner |
| REQ-HW-03 | The display shall be wired as in `Pins.md`: DIN→GPIO14, SCLK→GPIO13, CS→GPIO15, DC→GPIO27, RST→GPIO26, BUSY→GPIO25. | Stated |
| REQ-HW-04 | Four physical buttons shall be read by the firmware on GPIO16, 17, 18 and 19 (buttons 1 to 4). Each button connects its pin to GND; the firmware enables the internal pull-up and treats low as pressed. | Stated; pin choice derived from schematic (§3.1) |
| REQ-HW-05 | The firmware shall fit the board's resources: 4 MB flash, about 520 KB SRAM, no PSRAM. | Derived |
| REQ-HW-06 | The firmware shall not drive GPIO0, 2, 4 or 12, for the reasons in §3.1. | Derived from schematic |

### 3.1 Pin facts from the Rev 3 schematic

| Pins | On the board | Consequence |
|---|---|---|
| GPIO13, 14, 15, 25, 26, 27 | Wired to the panel connector as SCLK, SDI, CS, BUSY, RST, D/C | Matches `Pins.md`; not available for anything else |
| GPIO16, 17, 18, 19 | Unused on the board; broken out next to each other on header J4 (silk P16, P17, P18, P19) | Used for the four buttons |
| GPIO4 | Routed to the transistor that switches the panel's power rail. The series resistor (R35) is marked not fitted, so the rail is always on | Leave unused, so a board with R35 fitted keeps working |
| GPIO2 | Drives the on-board LED | Leave unused (also a boot-strapping pin) |
| GPIO0 | BOOT key, also driven by the auto-program circuit | Leave unused |
| GPIO12 | Boot-strapping pin (flash voltage) | Leave unused |
| GPIO34, 35, 36, 39 | Input-only, no internal pull-ups | Not suitable for buttons without external resistors |
| BS | Tied to GND through R11 | Panel is in 4-line SPI mode, as the firmware expects |

The board's USB-to-UART bridge is a CH343P. The board also has a two-position
DIP switch (SW1): position 1 connects the USB-UART bridge and must be on to
flash; position 2 selects the panel's drive resistor (A = 0.47 Ω, B = 3 Ω) and
must be set as Waveshare specifies for the 7.5" panel (OQ-8).

## 4. User experience

| ID | Requirement | Origin |
|---|---|---|
| REQ-UX-01 | The terminal shall present four pages: Home, Weather, News, Finance. | Stated |
| REQ-UX-02 | Buttons shall map directly to pages: 1→Home, 2→Weather, 3→News, 4→Finance. Any page is reachable with one press from any other page. | Stated |
| REQ-UX-03 | Pressing the button of the page already shown shall redraw that page (useful to clear ghosting and to confirm the device is alive). | Derived |
| REQ-UX-04 | Pages shall use graphics and icons (weather condition icons, trend arrows, charts, status glyphs), not text alone. | Stated |
| REQ-UX-05 | Layout shall be of shippable quality: a consistent grid and type scale, clear hierarchy, and no clipped or overlapping text. Text that does not fit shall be wrapped or truncated with an ellipsis. | Stated; criteria derived |
| REQ-UX-06 | Every page shall carry the same frame: page title, date, connectivity indicator, time the data was last updated, and a navigation strip showing which button is active. | Derived |
| REQ-UX-07 | On power-up the terminal shall show a splash/progress screen and then open the Home page without user action. | Derived |
| REQ-UX-08 | A button press shall start a display refresh within 500 ms. Presses made while the panel is refreshing shall be queued and collapsed so that the last one wins. | Derived |
| REQ-UX-09 | The display shall refresh only when visible content has changed. A full e-ink refresh takes several seconds and flashes, so needless refreshes are a defect. | Derived |

### 4.1 Home page

| ID | Requirement | Origin |
|---|---|---|
| REQ-HOME-01 | The Home page shall show highlights from the Weather, News and Finance pages. | Stated |
| REQ-HOME-02 | Highlights shall be: current conditions with today's high/low; the top headline of each news topic; and price with daily change for every configured symbol. | Derived |

### 4.2 Weather page

| ID | Requirement | Origin |
|---|---|---|
| REQ-WX-01 | The Weather page shall show a daily forecast. | Stated |
| REQ-WX-02 | The Weather page shall show an hourly forecast. | Stated |
| REQ-WX-03 | The Weather page shall show current conditions: temperature, feels-like, condition, humidity, wind, sunrise and sunset. | Derived |
| REQ-WX-04 | Weather data shall come from OpenWeatherMap `data/2.5/weather` and `data/2.5/forecast`. | Stated |
| REQ-WX-05 | Location, units (metric/imperial) and time zone shall be user-configurable. | Derived |
| REQ-WX-06 | The 2.5 `forecast` endpoint returns 3-hour steps over 5 days and no daily summary. The hourly view shall therefore show 3-hour steps, and the daily view shall be computed on the device from those steps (min/max temperature, dominant condition, precipitation probability). Accepted by the owner. | Derived |

### 4.3 News page

| ID | Requirement | Origin |
|---|---|---|
| REQ-NEWS-01 | Default topics shall be: US news/politics, global news/politics, finance and economics, technology. | Stated |
| REQ-NEWS-02 | Topics shall be user-configurable in a template file. | Stated |
| REQ-NEWS-03 | News shall come from NewsAPI `v2/top-headlines`. | Stated |
| REQ-NEWS-04 | Each headline shall show its title, source and age. Characters the fonts cannot draw shall be transliterated, never shown as boxes. | Derived |
| REQ-NEWS-05 | Requests shall send a `User-Agent` header; NewsAPI rejects requests without one. | Derived |

### 4.4 Finance page

| ID | Requirement | Origin |
|---|---|---|
| REQ-FIN-01 | Default instruments shall be DJIA, VOO, VXUS and VTI. | Stated |
| REQ-FIN-02 | Instruments shall be user-configurable, each with a ticker symbol and a display label. | Derived (from "topics configurable") |
| REQ-FIN-03 | Market data shall come from the Finnhub REST API (`/api/v1`). | Stated |
| REQ-FIN-04 | Each instrument shall show last price, absolute and percent change on the day, and the day's range. | Derived |
| REQ-FIN-05 | Each instrument should show an intraday trend chart built from quotes the device itself has sampled, because Finnhub's historical candles are not on the free tier. | Derived |
| REQ-FIN-06 | The display label shall be independent of the ticker so that an index can be represented by a proxy (for example label "DJIA", ticker `DIA`). The default instruments are examples; any symbol the provider's plan allows is acceptable. | Derived |

## 5. Configuration

| ID | Requirement | Origin |
|---|---|---|
| REQ-CFG-01 | User preferences (news topics, instruments, location, units) shall live in a user-editable file created from a committed template. | Stated |
| REQ-CFG-02 | API keys and Wi-Fi credentials shall live in a separate file created from a committed template. | Stated |
| REQ-CFG-03 | The real secrets file shall be excluded from git and from AI coding tools. | Stated |
| REQ-CFG-04 | API call limits shall be specified in a config file, per provider, as one or more windows (calls per minute, per day, per month). | Stated |
| REQ-CFG-05 | API URLs shall be defined in that config file and nowhere in source code. | Stated |
| REQ-CFG-06 | Configuration shall be validated when the firmware is built. A missing file, a placeholder key, a malformed URL or an unknown reference shall stop the build with a message naming the file, the key and the fix. | Derived |
| REQ-CFG-07 | Secrets shall never appear in logs or on the display. URLs shall be logged with credentials redacted. | Derived |
| REQ-CFG-08 | The project shall build and test without real secrets (for CI and for contributors) using the templates. | Derived |

## 6. API manager / scheduler

| ID | Requirement | Origin |
|---|---|---|
| REQ-API-01 | All API calls shall be made by one manager/scheduler component. No other component makes network requests. | Stated |
| REQ-API-02 | The scheduler shall never exceed the configured call limits, including after a restart. | Stated; restart clause derived |
| REQ-API-03 | The manager shall convert API responses into data models. | Stated |
| REQ-API-04 | The scheduler shall derive each job's refresh interval from the provider's configured limits, so that data is as fresh as the limits allow. Intervals are not hand-set. See §6.1. | Stated by owner (v0.2) |
| REQ-API-05 | Failed calls shall be retried with exponential backoff and jitter, honouring `Retry-After` when the provider sends it. | Derived |
| REQ-API-06 | Network activity shall never block button handling or rendering. | Derived |
| REQ-API-07 | The build shall print the derived schedule (interval and calls per day for every job) and shall fail if any job's derived interval is longer than its configured maximum acceptable staleness. | Derived |
| REQ-API-08 | All API traffic shall use HTTPS with server certificate validation. | Derived |
| REQ-API-09 | A provider's calls shall be evenly spaced in time across all of its jobs, not sent in bursts. | Stated by owner (v0.2) |
| REQ-API-10 | Call counts shall survive both restarts and power cycles, so that a daily or monthly limit cannot be exceeded by rebooting. | Derived |

### 6.1 How the schedule is derived

For each provider:

1. **Sustainable rate.** Convert every limit window to calls per second and
   take the lowest. (100 per day is the binding window for NewsAPI; 1,000,000
   per month is the binding window for OpenWeatherMap, not 60 per minute.)
2. **Reserve.** Multiply by a configurable utilisation factor (default 0.9). The
   remainder is kept for retries after failures.
3. **Share.** Divide the rate among the provider's jobs in proportion to a
   per-endpoint weight (default: equal).
4. **Floor.** If the result is faster than the endpoint's minimum useful
   interval, use that instead. Polling faster than the provider updates its
   data, or faster than the panel may refresh, buys nothing.
5. **Stagger.** Offset the jobs so the provider's calls are evenly spaced.

Because the calls are evenly spaced at or below the limit's average rate, any
rolling window contains no more than the limit. This holds whether the provider
counts per calendar day or per rolling 24 hours.

Adding or removing a topic or instrument changes the intervals automatically.
The rate limiter (REQ-API-02) remains as a hard guard behind the schedule.

## 7. Data model and rendering

| ID | Requirement | Origin |
|---|---|---|
| REQ-DM-01 | Rendering shall read only from the data model. Page code shall not perform I/O, parse API responses, or know which provider supplied the data. | Stated |
| REQ-DM-02 | Each piece of model data shall carry when it was fetched and the most recent error, so pages can show freshness and failures. | Derived |
| REQ-DM-03 | Data models shall be provider-neutral, so a provider can be replaced by changing one parser. | Derived |

## 8. Languages and safety

| ID | Requirement | Origin |
|---|---|---|
| REQ-LANG-01 | The application layer shall be written in Rust. | Stated |
| REQ-LANG-02 | The hardware abstraction layer shall be written in C. | Stated |
| REQ-LANG-03 | `unsafe` Rust shall be minimised: it is permitted only in the one crate that wraps the C HAL, and this shall be enforced automatically. | Stated; enforcement derived |
| REQ-LANG-04 | The C HAL shall expose a small, documented, blocking API: every function returns an error code, buffers are allocated by the caller, and C never calls back into Rust. | Derived |

## 9. Error handling

| ID | Requirement | Origin |
|---|---|---|
| REQ-ERR-01 | Errors shall be handled gracefully: a failure in one data source, or in the network as a whole, shall not stop the rest of the terminal from working. | Stated |
| REQ-ERR-02 | No error shall brick or lock the system. From any failure the device shall either recover by itself or reach a state the user can leave with a button press or a power cycle. | Stated |
| REQ-ERR-03 | Error messages shall say what failed, the likely cause, and what to do, with a stable error code, so the user can debug with minimal effort. | Stated; format derived |
| REQ-ERR-04 | When a refresh fails, the last good data shall stay on screen, marked as stale with its age. | Derived |
| REQ-ERR-05 | Panics and hangs shall be caught (panic handler, watchdog) and lead to a logged, automatic restart. | Derived |
| REQ-ERR-06 | Repeated crashes shall not become a reboot loop: after several consecutive abnormal restarts the device shall enter a safe mode that shows diagnostics and does not start the network. | Derived |
| REQ-ERR-07 | Diagnostics shall be available over the USB serial log and on a diagnostics screen (firmware version, network state, per-source status, API budget used, last errors). | Derived |
| REQ-ERR-08 | The device shall always remain re-flashable over USB. The firmware shall not burn eFuses, enable secure boot or flash encryption, or modify the bootloader. | Derived |

## 10. Display care

These follow the panel vendor's handling guidance and protect the hardware.

| ID | Requirement | Origin |
|---|---|---|
| REQ-DISP-01 | The panel shall be put into deep sleep after every refresh and never left powered. | Derived |
| REQ-DISP-02 | The panel shall receive a full refresh at least once every 24 hours. | Derived |
| REQ-DISP-03 | Refreshes caused by new data (not by a button) shall be at least 180 seconds apart. | Derived |

## 11. Non-functional

| ID | Requirement | Origin |
|---|---|---|
| REQ-PERF-01 | Composing a page into the frame buffer shall take no more than 250 ms on the ESP32. | Derived |
| REQ-PERF-02 | At least 40 KB of heap shall remain free at the worst point of normal operation (during a TLS request). | Derived |
| REQ-PERF-03 | The release image shall fit the application partition with at least 10 % spare. | Derived |
| REQ-REL-01 | The terminal shall run unattended for at least 7 days, through Wi-Fi and API outages, without a manual restart. | Derived |
| REQ-TEST-01 | Everything that does not touch hardware (models, parsers, scheduler, layout, rendering) shall build and be unit-tested on a development PC. | Derived |
| REQ-TEST-02 | Every page shall be renderable to an image on a PC and covered by golden-image tests. | Derived |
| REQ-TEST-03 | Continuous integration shall run formatting, lints, host tests and a firmware build on every change. | Derived |
| REQ-MAINT-01 | Pin assignments shall be defined in one header in the C HAL. | Derived |
| REQ-MAINT-02 | The repository shall document setup, wiring, configuration and troubleshooting (including the error-code list). | Derived |

## 12. Assumptions

- **A-1** The terminal is powered from USB. Battery operation is not a goal.
- **A-2** Changing configuration means editing a file and re-flashing.
- **A-3** One Wi-Fi network, 2.4 GHz, WPA2/WPA3-Personal.
- **A-4** Free tiers of all three providers are used. Limits as published in
  November 2026 (see §12.1).
- **A-5** English-language content; fonts cover Latin-1.

### 12.1 Provider limits

| Provider | Free-plan limit | Source | Resulting default schedule |
|---|---|---|---|
| OpenWeatherMap | 60 calls/minute and 1,000,000 calls/month | [Pricing](https://openweathermap.org/full-price) | Limit allows a call every few seconds; the 10-minute floor applies (OpenWeatherMap updates current data about that often). Current: every 10 min. Forecast: every 30 min. |
| NewsAPI (Developer) | 100 requests/day, no overage | [Pricing](https://newsapi.org/pricing) | Limit-bound. 90 calls/day over 4 topics: each topic every 64 min, one news call every 16 min. |
| Finnhub | 60 calls/minute | [Pricing](https://finnhub.io/pricing) | Limit allows each of 4 symbols every 4.4 s; the 60-second floor applies. |

Not confirmed from the providers' own pages: that NewsAPI's free plan delays
articles by 24 hours; that Finnhub also caps all plans at 30 calls/second; and
that Finnhub's free plan excludes index quotes. None of these changes the
design. The limits are config values, so a plan change is an edit to
`config/api.toml`.

## 13. Open questions

| ID | Question | Status |
|---|---|---|
| OQ-1 | Panel variant. | **Resolved**: 800×480 black/white, confirmed by owner. Which refresh modes this revision supports is still checked on the bench (plan task J04). |
| OQ-2 | Button GPIOs. | **Resolved** from the schematic: GPIO16, 17, 18, 19 (§3.1). |
| OQ-3 | DJIA is an index and may not be quotable on Finnhub's free plan. | **Resolved**: instruments are examples; use the DIA ETF, labelled "DJIA (DIA)". |
| OQ-4 | 3-hourly rather than hourly forecast. | **Resolved**: accepted. |
| OQ-5 | Provider limits. | **Resolved**: researched (§12.1); the scheduler derives intervals from them (§6.1). Residual: NewsAPI's Developer plan is licensed for development and testing only, so a unit used day-to-day or shipped to others needs a paid plan or a different news provider. |
| OQ-6 | Should settings be changeable without re-flashing (captive portal, serial console)? | Open. Out of scope for v1. |
| OQ-7 | Secrets are stored unencrypted in flash; anyone with the device can read them. | Open. Accepted for v1 and documented. |
| OQ-8 | Which position of SW1-2 (panel drive resistor, A or B) does Waveshare specify for the 7.5" panel? | **Resolved** on the bench (2026-11-18): the board's current setting drives the panel correctly; a test pattern displays with full contrast (plan task C07). |

## 14. Out of scope for v1

- Over-the-air updates.
- Runtime configuration or provisioning UI.
- Battery power and deep-sleep power management.
- Colour or greyscale rendering.
- Localisation.
- Keeping fetched data across a power cycle.
- A market-holiday calendar.
