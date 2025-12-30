Raise Ceiling: +
Lower Floor:   -
Both:          *

General:
  - Adjust bottom note without changing key
  - Pulse sometimes gets removed from queue while held
  * Alt chording mode: 1-octave piano-style keyboard for note inclusion
    - Certain buttons like change key, pedal, pulse, etc. would still need dedicated buttons.
  + Pedal toggle/mod?

Android app:
* Prevent swiping off the left/right sides closing the app (forward/backward button)
* Add Pulse
* Make notes sound less derpy (attack envelope?)
* Dedupe rapid repeats
* Better audio out device
* Octave up/down split or modifier
- Add more state to display
* Larger screen (larger android device? iOS?)

Desktop app:
* Currently the top note is Ti when the bottom is Do. Can we fit one more?
- Add state to display
* Find large multi-touch trackpad

Refactor flow prompts:
* Introduction:
* DRY:
Here's a rust crate providing a musical instument desktop and android app. We can't build the android app locally (no gradle), but we build that with github actions later.
Please investigate and identify a few key areas that could be refactored to reduce repetition in the codebase.
* Modularity:
Here's a rust crate providing a musical instument desktop and mobile app. We can't build the android app locally (no gradle), but we build that with github actions later. The system python version is python2.
Please investigate and identify a few key areas that could be refactored to improve modularity of the codebase.
* Idiomatic
Here's a rust crate providing a musical instument desktop and mobile app.
After taking a look around the codebase, does anything stand out to you as a missed opportunity for me to write more idiomatic rust?
* Comments
Here's a rust crate providing a musical instument desktop and mobile app.
Take a look around the codebase to identify and fix any comments that appear be incorrect, out of date, or unnecessary. We'd prefer to let the code be self-documenting as much as possible, so find opportunities to improve naming if that can increase clariy and reduce the need for comments.
* Unit tests
After taking a look at the code and tests, does it seem that tests are missing for any key functionality?
* Obviously missing features?

* Afterward:
  * Was it a good suggestion? Take it.
  * Repeatedly mid suggestions? Abort.
  * Run rustfmt and commit.

copilot --allow-tool "write" --allow-tool "shell(cargo check:*)" --allow-tool "shell(cargo test:*)" 
