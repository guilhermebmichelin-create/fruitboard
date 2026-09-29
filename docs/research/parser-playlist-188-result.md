# Saved playlist pattern clips (issue 188)

The approved F13 synthetic FL Studio 2026 project was saved by operating the
matching 26.1.0.5530 GUI. Its registered playlist view shows four one-bar
pattern clips on Track 1 in order A, A, B, C across bars 1–4. The screenshot
and raw save stay outside Git; the owner-approved sanitized FLP is in the
ordinary corpus. Its 47,449 bytes have SHA-256
`227b44e0a409e067f59d0e21da7c187537e242e45c48ad015e9396aadc005c2a`.
No new project or fixture was made for this field.

Bounded, read-only inspection of F13 found one event 233 at byte offset 1826
with a 352-byte payload: four consecutive 88-byte records. The two approved
minimal 2026 projects, F11 and F12, each have one empty event-233 payload.
Within F13, little-endian values in each record are:

| Record | Bytes 0–3: start | Bytes 4–7: item | Bytes 8–11: length | Bytes 12–15: track token | GUI clip |
| --- | ---: | --- | ---: | ---: | --- |
| 1 | 0 | `0x50015000` | 384 | 499 | A |
| 2 | 384 | `0x50015000` | 384 | 499 | A |
| 3 | 768 | `0x50025000` | 384 | 499 | B |
| 4 | 1152 | `0x50035000` | 384 | 499 | C |

The low 16 bits of the item are `0x5000`; its high 16 bits are `0x5000` plus
the saved pattern ID. This interpretation exactly matches the approved GUI
view and the three IDs and names already validated by PRs #185 and #187. The
88-byte record and its remaining fields are not a general FLP specification.

`playlistPatternClips` exposes only the matched pattern ID, start and length in
stored ticks, and the raw track token. It keeps duplicate placements. The
event stream's 384-tick spacing matches the four bars in this project; the
parser does not yet convert ticks to bars or seconds, interpret token 499 as a
normalized track number, or claim an overall song length. One fixture does not
prove edited time signatures, tempo changes, other tracks, audio/automation
clips, or multiple arrangements. The field is therefore gated to the one
verified saved build and marks unfamiliar cases unsupported.

The focused tests check exact F13 values and unchanged bytes, the empty F11/F12
payloads, and synthetic unsupported and bounded failure cases. Synthetic cases
exercise defensive behavior; they are not additional GUI compatibility proof.
The parser process remains read-only and limited by its existing 4 MiB input,
2 MiB event, and 256 KiB response caps, plus a 1024-clip field cap. No scanner,
database, plugin, or packaging path is activated by this field.
