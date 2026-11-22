# A Workday Storyline with Safe Your Eyes

![A workday with Safe Your Eyes](assets/storyline.gif)

Illustration drawn with [tinycharts](https://github.com/tunglt1810/tinycharts).
Each panel below matches one beat in the nine-panel comic strip above.
Read the technical details in [the specification](specs/SPEC.md).

## 1. Morning at the Desk

Morning sunlight warms your desk beside a hot coffee mug and a curled-up cat.
You run `safe-your-eyes` in your terminal.
The single-instance daemon starts in the system tray.
It uses 2 to 4 MB of RAM and 0% GPU because it creates no graphics context.

## 2. Fifteen Minutes of Focus

You write code for 15 minutes while the wall clock ticks forward.
In the background, `Scheduler` counts down the 900-second `Short` break interval.
Your screen stays clear, and the daemon keeps graphics memory at zero.

## 3. Ten-Second Notice and Chime

Ten seconds before the break, a desktop notification appears in the corner of your screen.
At the same time, the daemon plays a soft synthesized two-tone chime (`PreBreak`, C5 to E5).
You finish your current line of code and lift your hands from the keyboard.

## 4. Cozy Dark-Roast Overlay and 2-Second Lock

When the countdown reaches `00:00`, the daemon starts a native child process for the break overlay.
A dark-roast canvas (`#18140E`) fades in around a `#231D16` card with warm cream text (`#FFFEF7`).
The `00:15` countdown starts in warm coral (`#EE8F64`).
For the first 2 seconds, `shortcut_disable_secs` locks `Postpone (Space)` and `Skip (Esc)` so stray keystrokes never close the break.

## 5. Lock Opens: Rest or Skip

After 2 seconds, `Postpone (Space)` and `Skip (Esc)` unlock.
You decide between two actions on the break card.
You can rest your eyes for 15 seconds, or you can skip to stay on a deadline.

## 6. Detour: Skip Limit at 2/2

Today you press `Skip` or `Postpone` twice in a row.
On the next break, `LimitConsecutiveSkipping` hits `2/2` and locks both buttons.
The overlay shows `Skipped or postponed 2/2 breaks in a row`.
This detour returns you straight to the break card so your eyes get real rest.

## 7. Look Far Away for 15 Seconds

You turn toward the window and focus on distant green trees for 15 seconds.
When the timer reaches `00:00`, a resolving chime (`PostBreak`, G5 to C5) plays and resets consecutive skips to `0`.
The overlay child process exits immediately, and the operating system releases all graphics memory (`0% GPU`).

## 8. Four Short Breaks, One Long Break

After four short breaks and 75 minutes of work, `BreakQueue` stages a 60-second `Long` break.
The overlay prompts you to stand up, stretch your back, and refill your tea.
A full minute away from the screen rests tired eyes and stiff shoulders.

## 9. Clear Eyes at Dusk and Session Persistence

Sunset colors the window at the end of the workday, and your eyes still feel clear.
You run `safe-your-eyes --quit` to stop the daemon cleanly.
When you enable `persist_state`, the daemon writes your `BreakQueue` position to `session.toml` before exit.
Next morning, the daemon resumes right from your saved break position.
