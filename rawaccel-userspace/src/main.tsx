import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";
import { MotionConfig } from 'framer-motion';
import { MouseTelemetryProvider } from './components/MouseTelemetry';

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MotionConfig reducedMotion="user"><MouseTelemetryProvider><App /></MouseTelemetryProvider></MotionConfig>
  </React.StrictMode>,
);
