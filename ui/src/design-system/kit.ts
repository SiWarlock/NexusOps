// Single import surface for the NexusOps-ui-kit components the shell uses.
//
// Keeps the @ui-kit alias path in ONE place so a re-hue / kit upgrade touches
// only this file (the kit's "re-hue touches primitives only" promise, §11.1).
// Tokens are linked separately via the kit styles.css (imported in main.tsx);
// components come from the kit .jsx sources through the @ui-kit Vite alias.
export { Button } from "@ui-kit/controls/Button";
