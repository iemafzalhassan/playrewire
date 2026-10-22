<!-- if-else: a road forks at a question. The answer is yes, so the car takes
     the "if" road to Bright street; the "else" road stays empty. -->
<script lang="ts">
  import { roughRect } from "../lib/sketch";

  const board = roughRect(170, 112, 140, 40, 41);
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="A road comes to a fork with a sign asking: lights greater than 2? The answer is yes, so a car takes the if road up to Bright street, where a lamp glows. The else road to Dark street stays empty."
>
  <!-- roads: trunk, the yes road (left) and the no road (right) -->
  <path class="paper a-fade" style:--d="0s" d="M222 270 L226 176 C200 140 150 90 72 52 L104 30 C170 66 220 110 240 150 C262 110 312 66 378 30 L410 52 C330 90 282 140 256 176 L260 270 Z" />
  <g class="wob">
    <path class="ink a-draw" pathLength="1" style:--d="0.1s" d="M222 270 L226 176 C200 140 150 90 72 52" />
    <path class="ink a-draw" pathLength="1" style:--d="0.2s" d="M260 270 L256 176 C282 140 330 90 410 52" />
    <path class="ink a-draw" pathLength="1" style:--d="0.35s" d="M104 30 C170 66 220 110 240 150 C262 110 312 66 378 30" />
  </g>
  <!-- the chosen road is inked in, the other stays pencil -->
  <path class="ink bold accent-line yes-trail a-draw" pathLength="1" style:--d="3s" style:--t="1.2s" d="M241 262 L241 180 C220 140 160 90 92 48" />
  <path class="ink thin pencil" d="M241 180 C262 140 320 90 392 48" />

  <!-- the question sign -->
  <path class="ink a-draw" pathLength="1" style:--d="0.6s" d="M240 152 L241 176" />
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.5s" d={board} />
  </g>
  <text class="mono mid a-fade" style:--d="0.9s" x="240" y="138" font-size="15" font-weight="700">lights &gt; 2 ?</text>

  <!-- branch labels -->
  <g class="a-fade" style:--d="1.1s">
    <text class="hand" x="112" y="118" font-size="26">yes</text>
    <text class="mono" x="100" y="138" font-size="13">if</text>
    <text class="hand" x="330" y="118" font-size="26" opacity="0.6">no</text>
    <text class="mono" x="334" y="138" font-size="13" opacity="0.6">else</text>
  </g>
  <text class="hand a-fade" style:--d="1.2s" x="18" y="28" font-size="20">Bright street</text>
  <text class="hand a-fade" style:--d="1.2s" x="372" y="22" font-size="20" opacity="0.55">Dark street</text>

  <!-- Bright street's lamp comes on when the car arrives -->
  <g class="a-pop" style:--d="4s">
    <circle class="glow" cx="40" cy="56" r="22" opacity="0.6" />
    <path class="ink thin" d="M40 34 L40 26 M20 46 L13 41 M60 46 L67 41" />
  </g>
  <circle class="ink glow a-fade" style:--d="4s" cx="40" cy="56" r="10" />
  <path class="ink" d="M40 66 L41 100" />
  <circle class="ink card" cx="446" cy="50" r="10" />
  <path class="ink soft" d="M446 60 L445 94" />

  <!-- the car -->
  <g class="car">
    <g transform="rotate(-50)">
      <rect class="ink accentfill" x="-11" y="-17" width="22" height="34" rx="6" />
      <rect class="ink thin card" x="-7" y="-11" width="14" height="9" rx="2" />
    </g>
  </g>
</svg>

<style>
  .car {
    transform: translate(122px, 72px);
    animation: drive 3.4s ease-in-out 1.2s both;
  }
  .car > g {
    /* points up the trunk first, then turns onto the yes road */
    animation: turn 3.4s ease-in-out 1.2s both;
  }
  @keyframes drive {
    0% {
      transform: translate(241px, 262px);
    }
    35%,
    55% {
      transform: translate(241px, 192px);
    }
    100% {
      transform: translate(122px, 72px);
    }
  }
  @keyframes turn {
    0%,
    55% {
      transform: rotate(0deg);
    }
    68%,
    100% {
      transform: rotate(-50deg);
    }
  }
</style>
