// See-step scenes by id. Every scene id used in content/go must be here
// (scenes.test.ts checks the real lesson files).
import type { Component } from "svelte";
import "./scene.css";
import BinaryLamps from "./BinaryLamps.svelte";
import BusRoad from "./BusRoad.svelte";
import ComputerInside from "./ComputerInside.svelte";
import ForkRoad from "./ForkRoad.svelte";
import LabeledBoxes from "./LabeledBoxes.svelte";
import LampStreet from "./LampStreet.svelte";
import Machine from "./Machine.svelte";
import PrintSay from "./PrintSay.svelte";
import TrainCars from "./TrainCars.svelte";

export const scenes: Record<string, Component> = {
  "computer-inside": ComputerInside,
  "bus-road": BusRoad,
  "binary-lamps": BinaryLamps,
  "print-say": PrintSay,
  "labeled-boxes": LabeledBoxes,
  "fork-road": ForkRoad,
  "lamp-street": LampStreet,
  "train-cars": TrainCars,
  machine: Machine,
};
