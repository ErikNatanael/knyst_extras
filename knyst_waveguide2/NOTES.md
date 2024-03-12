# Tuning

## Split string

The following aspects need to be true at the same time:
- octaves are precisely in tune
- dampening does not affect tuning (except extremely close)
- stop amount does not affect tuning relative to non-stopped pitches


These parameters are available:
- lpf compensation amount/curve
- constant offset to delay lines
- different offsets for different delays

When the damping freq is closer to the fundamental it seems to push low pitches down and high pitches up.

50. down
100. down
200. down
300. up and then down when very close
400. up and then down when very close
800. up
1200. up

### Stop amount

In order to keep the 