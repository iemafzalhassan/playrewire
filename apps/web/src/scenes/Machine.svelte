<!-- functions: a little machine with a name. 4 goes in, the gears turn, and 8
     comes back out. -->
<script lang="ts">
  import { roughRect } from "../lib/sketch";

  const box = roughRect(150, 70, 180, 130, 201);
  const plate = roughRect(184, 160, 112, 30, 202, 1);
</script>

<svg
  class="scene"
  viewBox="0 0 480 270"
  role="img"
  aria-label="A machine with the name double on its front. The number 4 drops into the funnel on top, the gears inside turn, and 8 slides out of the chute on the side. A function is a little machine: something goes in and a result comes back."
>
  <!-- funnel and chute -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" style:--d="0.2s" d="M164 70 L150 26 L230 26 L214 70 Z" />
    <path class="ink card a-draw" pathLength="1" style:--d="0.35s" d="M330 150 L420 182 L420 210 L330 182 Z" />
  </g>

  <!-- 4 goes in -->
  <g class="in">
    <rect class="ink card" x="-20" y="-16" width="40" height="32" rx="6" />
    <text class="mono mid" y="7" font-size="20" font-weight="700">4</text>
  </g>

  <!-- body -->
  <g class="wob">
    <path class="ink card a-draw" pathLength="1" d={box} />
    <path class="ink thin paper a-draw" pathLength="1" style:--d="0.5s" d={plate} />
  </g>
  <g transform="translate(206 112)">
    <circle class="ink gear-teeth spin" r="22" />
    <circle class="ink card" r="14" />
    <circle class="inkfill" r="4" />
  </g>
  <g transform="translate(262 108)">
    <circle class="ink gear-teeth spin back" r="16" />
    <circle class="ink card" r="9" />
    <circle class="inkfill" r="3" />
  </g>
  <text class="hand mid" x="240" y="183" font-size="24">double</text>
  <text class="mono mid a-fade" style:--d="0.9s" x="240" y="228" font-size="13">func double(n int) int</text>
  <text class="mono mid a-fade" style:--d="1.1s" x="240" y="248" font-size="13">return n * 2</text>

  <!-- 8 comes out -->
  <g transform="translate(448 222)">
    <g class="out">
      <rect class="ink accentfill" x="-20" y="-16" width="40" height="32" rx="6" />
      <text class="mono mid on-ink" y="7" font-size="20" font-weight="700">8</text>
    </g>
  </g>

  <!-- notes -->
  <g class="a-fade" style:--d="0.8s">
    <text class="note" x="14" y="42" font-size="22">4 goes in</text>
    <path class="ink thin accent-line" d="M100 36 C112 31 124 29 140 30 M131 24 L141 30 L132 36" />
    <text class="note" x="350" y="134" font-size="22">8 comes out</text>
  </g>
  <text class="hand a-fade" style:--d="3.4s" x="22" y="140" font-size="22">double(4)</text>
  <text class="hand a-fade" style:--d="3.6s" x="22" y="166" font-size="22">gives back <tspan class="note">8</tspan></text>
</svg>

<style>
  .gear-teeth {
    fill: none;
    stroke-width: 7;
    stroke-dasharray: 5 4.2;
  }
  .spin {
    transform-box: fill-box;
    transform-origin: center;
    animation: spin 0.9s linear 1.9s 2 both;
  }
  .spin.back {
    animation-name: spin-back;
  }
  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes spin-back {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(-360deg);
    }
  }
  .in {
    opacity: 0;
    animation: feed 1.4s ease-in 0.8s both;
  }
  @keyframes feed {
    0% {
      opacity: 0;
      transform: translate(190px, -30px);
    }
    15% {
      opacity: 1;
    }
    55% {
      transform: translate(190px, 20px);
    }
    100% {
      opacity: 1;
      transform: translate(190px, 76px) scale(0.6);
    }
  }
  .out {
    animation: chute 1s cubic-bezier(0.3, 0.6, 0.4, 1) 3s both;
  }
  @keyframes chute {
    from {
      opacity: 0;
      transform: translate(-110px, -40px);
    }
    20% {
      opacity: 1;
    }
  }
</style>
