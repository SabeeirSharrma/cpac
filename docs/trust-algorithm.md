# CPAC Week 1 Trust Algorithm

CPAC assigns every resolved package a score from 0 to 100. Week 1 uses package metadata that is available from `pacman -Si` and the AUR RPC API.

The score is explainable: `cpac trust <package>` prints each signal, the points it contributed, and the reason.

## Signals

| Signal | Max | Notes |
|---|---:|---|
| Repository source | 30 | Official packages receive the strongest source score. AUR packages receive a smaller community-source score. |
| Package age | 15 | Older packages receive more points. Official packages receive a conservative default because pacman metadata does not expose first-submitted dates. |
| Maintainer | 15 | Maintained packages score higher. Orphaned AUR packages are penalized. |
| Popularity | 15 | AUR vote counts are used when available. Official packages receive a conservative default. |
| Last updated | 15 | Recently updated packages score higher. Official packages receive a conservative default. |
| Out-of-date flag | -10 | AUR packages flagged out-of-date lose points. |

## Recommendations

| Score | Recommendation |
|---:|---|
| 80-100 | Safe |
| 60-79 | Moderate |
| 40-59 | Caution |
| 20-39 | Warning |
| 0-19 | Danger |

## Scope

This is intentionally a Week 1 scoring model. Later phases can add build history, package integrity checks, advisory databases, local audit history, and maintainer history from a CPAC trust database.
