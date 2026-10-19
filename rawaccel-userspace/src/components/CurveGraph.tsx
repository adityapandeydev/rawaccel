import React, { useMemo, useState, useId } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { AccelArgs } from '../types';
import { generateGraphData, createEvaluator, GraphPoint } from '../math';
import { trackerInputs } from '../tracker';
import { useMouseTelemetry } from './MouseTelemetry';
import { Zap, Activity, TrendingUp } from 'lucide-react';

interface CurveGraphProps {
  argsX: AccelArgs;
  argsY: AccelArgs;
  visibleGraphs: string[];
  isExpanded: boolean;
  whole: boolean;
  onUpdateLookupPoint?: (axis: "x" | "y", newTable: {x: number, y: number}[]) => void;
}

export function CurveGraph({ 
  argsX, 
  argsY, 
  visibleGraphs,
  isExpanded, whole,
  onUpdateLookupPoint
}: CurveGraphProps) {

  const plotId = useId();
  const { mouseSpeed, trackerUnavailable } = useMouseTelemetry();
  const [dragging, setDragging] = useState<{ axis: "x"|"y", index: number } | null>(null);

  const maxSpeed = useMemo(() => {
    let m = isExpanded ? 200 : 80;
    if (argsX.cap_mode === "in" && argsX.cap.x > 0) m = Math.max(m, argsX.cap.x * 1.2);
    if (argsY.cap_mode === "in" && argsY.cap.x > 0) m = Math.max(m, argsY.cap.x * 1.2);
    for (const table of [argsX.lookup_table, argsY.lookup_table]) {
      for (const point of table ?? []) {
        if (Number.isFinite(point.x) && point.x > 0) m = Math.max(m, point.x * 1.2);
      }
    }
    return Math.ceil(m / 20) * 20;
  }, [argsX, argsY, isExpanded]);

  const dataX = useMemo(() => generateGraphData(argsX, maxSpeed, 200), [argsX, maxSpeed]);
  const dataY = useMemo(() => generateGraphData(argsY, maxSpeed, 200), [argsY, maxSpeed]);

  const isSame = JSON.stringify(argsX) === JSON.stringify(argsY);

  const getBounds = (key: keyof GraphPoint, isVelocity: boolean = false) => {
    const allVals = [...dataX.map(d => d[key] as number), ...dataY.map(d => d[key] as number)].filter(n => isFinite(n) && !isNaN(n));
    if (allVals.length === 0) return { min: 0, max: 1 };
    
    let min = Math.min(...allVals);
    let max = Math.max(...allVals);
    
    if (isVelocity) {
      min = 0;
      // Add 5% clearance at the top to prevent clipping the stroke
      max = Math.ceil((max * 1.05) / 20) * 20;
      if (max < 100) max = 100;
    } else {
      let pad = (max - min) * 0.15;
      if (pad === 0) {
        pad = max === 0 ? 0.1 : Math.abs(max * 0.1);
      }
      min = min - pad;
      max = max + pad;
      
      if (min < 0) min = 0;
      if (min > 0.9 && min <= 1.0) min = 0.9;
      if (max <= min) max = min + 1;
      
      // Magic: Ensure the starting point is at most in the middle of the Y-axis
      const startVal = dataX.length > 0 ? (dataX[0][key] as number) : 1.0;
      const requiredMax = 2 * startVal - min;
      if (max < requiredMax) {
        max = requiredMax;
      }
    }
    
    return { min, max };
  };

  const bounds = {
    sensitivity: getBounds('sensitivity'),
    velocity: getBounds('velocity', true),
    gain: getBounds('gain')
  };

  const generatePath = (data: GraphPoint[], key: keyof GraphPoint, minY: number, maxY: number) => {
    if (data.length === 0) return "";
    const range = maxY - minY;
    if (range <= 0) return "";
    
    const mapPoint = (x: number, y: number) => {
      const px = (x / maxSpeed) * 100;
      let py = 100 - ((y - minY) / range) * 100;
      if (py < 0) py = 0;
      if (py > 100) py = 100;
      return `${px},${py}`;
    };
    
    const start = mapPoint(data[0].x, data[0][key] as number);
    const path = [`M ${start}`];
    for (let i = 1; i < data.length; i++) {
      path.push(`L ${mapPoint(data[i].x, data[i][key] as number)}`);
    }
    return path.join(" ");
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!dragging || !onUpdateLookupPoint) return;
    const args = dragging.axis === "x" ? argsX : argsY;
    if (!args.lookup_table || args.lookup_table.length < 2) return;

    // Use sensitivity bounds because that's what we plot lookup tables as
    const { min: minY, max: maxY } = bounds.sensitivity;

    // currentTarget is the sensitivity graph overlay that received the event.
    // This avoids a shared ref accidentally measuring another visible graph.
    const rect = e.currentTarget.getBoundingClientRect();
    const px = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const py = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));

    const requestedX = px * maxSpeed;
    const range = maxY - minY;
    const newY = minY + (1 - py) * range;

    const table = [...args.lookup_table];
    const index = dragging.index;
    
    // Constrain X so points cannot cross neighbors
    const minX = index > 0 ? table[index - 1].x + 0.001 : 0.001;
    const maxX = index < table.length - 1 ? table[index + 1].x - 0.001 : maxSpeed;
    const newX = Math.max(minX, Math.min(maxX, requestedX));
    const displayedSensitivity = Math.max(minY, Math.min(maxY, newY));
    
    table[index] = {
      x: newX,
      // Velocity LUTs store output velocity; sensitivity LUTs store sensitivity.
      y: args.gain ? displayedSensitivity * newX : displayedSensitivity
    };
    
    onUpdateLookupPoint(dragging.axis, table);
  };

  const handlePointerUp = () => {
    if (dragging) {
      setDragging(null);
    }
  };

  const renderGraph = (
    title: string, 
    icon: React.ReactNode, 
    dataKey: keyof GraphPoint, 
    yLabel: string, 
    color: string
  ) => {
    const { min: minY, max: maxY } = bounds[dataKey as 'sensitivity' | 'velocity' | 'gain'];
    const pathX = generatePath(dataX, dataKey, minY, maxY);
    const pathY = generatePath(dataY, dataKey, minY, maxY);

    // Fixed Y ticks (5 intervals)
    const yTicks = [0, 0.2, 0.4, 0.6, 0.8, 1.0];
    
    // Dynamic X ticks based on maxSpeed (intervals of 10)
    const numXTicks = maxSpeed / 10;
    const xTicks = Array.from({ length: numXTicks + 1 }, (_, i) => i / numXTicks);

    return (
      <motion.div 
        key={dataKey}
        className="graph-card" aria-label={`${title} graph`}
        layout
        initial={{ opacity: 0, scale: 0.95, y: -20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: -20 }}
        transition={{ type: "spring", stiffness: 300, damping: 30 }}
        style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}
      >
        <div className="graph-caption" style={{
          display: "flex", 
          alignItems: "center", 
          justifyContent: "space-between",
          gap: "0.5rem", 
          marginBottom: "0.4rem", 
          marginTop: 0,
          color: "var(--text-main)", 
          fontSize: "0.9rem", 
          fontWeight: 700 
        }}>
          <span className="graph-title">{icon}{title}</span><span className="graph-unit">{yLabel}</span>
        </div>
        <div className="graph-plot">
          <div className="graph-axis-title">{title === "Sensitivity" ? "Sensitivity (×)" : yLabel}</div>
          
          <div style={{ flex: 1, position: "relative" }}>
            {/* Grid Lines & Labels (Unscaled) */}
            <svg width="100%" height="100%" style={{ overflow: "visible", position: "absolute", top: 0, left: 0 }}>
              {/* Fine grid and Tiered transition guides are display-only. */}
              <defs><pattern id={`${plotId}-grid-${dataKey}`} width="20" height="20" patternUnits="userSpaceOnUse"><path d="M 20 0 H 0 V 20" fill="none" stroke="var(--border)" opacity="0.3" /></pattern></defs>
              <rect width="100%" height="100%" fill={`url(#${plotId}-grid-${dataKey})`} />
              {isSame && argsX.mode === 'tiered' && (argsX.t_type || 'linear') === 'linear' && [
                [argsX.tiered_input_offset1, argsX.tiered_transition1], [argsX.tiered_input_offset2, argsX.tiered_transition2],
              ].map(([start, width], index) => <g key={`stage-${index}`}>
                <rect x={`${start / maxSpeed * 100}%`} width={`${Math.max(0, Math.min(width, maxSpeed - start)) / maxSpeed * 100}%`} height="100%" fill={color} opacity="0.055" />
                {[start, start + width].filter(x => x >= 0 && x <= maxSpeed).map((x, i) => <line key={i} x1={`${x / maxSpeed * 100}%`} x2={`${x / maxSpeed * 100}%`} y1="0" y2="100%" stroke="var(--text-muted)" strokeDasharray="4 4" opacity="0.6" />)}
              </g>)}
              {/* Y Axis Grid */}
              {yTicks.map(p => {
                const isZero = p === 0;
                const yPos = `${100 - (p * 100)}%`;
                const yVal = minY + p * (maxY - minY);
                return (
                  <g key={`grid-y-${p}`}>
                    {!isZero && <line x1="0" y1={yPos} x2="100%" y2={yPos} stroke="var(--border)" strokeWidth="1" />}
                    <text x="-10" y={yPos} fill="var(--text-muted)" fontSize="10" fontWeight="500" dominantBaseline="middle" textAnchor="end">
                      {yVal.toFixed(dataKey === 'velocity' ? 0 : 2)}
                    </text>
                  </g>
                );
              })}
              
              {/* X Axis Grid */}
              {xTicks.map((p, i) => {
                const isZero = p === 0;
                const xPos = `${p * 100}%`;
                // To prevent crowding on large domains, only label every 2nd tick if very expanded, or skip some
                const showLabel = numXTicks <= 10 || (i % 2 === 0);
                
                return (
                  <g key={`grid-x-${p}`}>
                    {!isZero && <line x1={xPos} y1="0" x2={xPos} y2="100%" stroke="var(--border)" strokeWidth="1" />}
                    {showLabel && (
                      <text x={xPos} y="100%" dy="16" fill="var(--text-muted)" fontSize="10" fontWeight="500" textAnchor="middle">
                        {(p * maxSpeed).toFixed(0)}
                      </text>
                    )}
                  </g>
                );
              })}
              
              {/* Main Axes */}
              <line x1="0" y1="0" x2="0" y2="100%" stroke="var(--text-main)" strokeWidth="1" />
              <line x1="0" y1="100%" x2="100%" y2="100%" stroke="var(--text-main)" strokeWidth="1" />
            </svg>

            {/* The Curve Layer (Scaled) */}
            <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none" style={{ overflow: "visible", position: "absolute", top: 0, left: 0, pointerEvents: "none" }}>
              <defs><linearGradient id={`${plotId}-${dataKey}`} x1="0" x2="0" y1="0" y2="1"><stop offset="0%" stopColor={isSame ? color : 'var(--color-x)'} stopOpacity="0.18" /><stop offset="100%" stopColor={isSame ? color : 'var(--color-x)'} stopOpacity="0.025" /></linearGradient></defs>
              <path d={`${pathX} L 100,100 L 0,100 Z`} fill={`url(#${plotId}-${dataKey})`} stroke="none" />
              {/* The X Curve */}
              <path d={pathX} fill="none" stroke={isSame ? color : "var(--color-x)"} strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
              
              {/* The Y Curve */}
              {!isSame && (
                <path d={pathY} fill="none" stroke="var(--color-y)" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" strokeDasharray="6 6" vectorEffect="non-scaling-stroke" />
              )}
            </svg>
            
            {/* Unscaled Overlay for Tracker Dots and Draggable Nodes */}
            <svg 
              width="100%" height="100%" 
              style={{ overflow: "visible", position: "absolute", top: 0, left: 0, pointerEvents: dragging ? "auto" : "none" }}
              onPointerMove={handlePointerMove}
              onPointerUp={handlePointerUp}
            >
              {dataKey === 'sensitivity' && isSame && argsX.mode === 'tiered' && (argsX.t_type || 'linear') === 'linear' && Array.from(new Set([
                argsX.tiered_input_offset1, argsX.tiered_input_offset1 + argsX.tiered_transition1,
                argsX.tiered_input_offset2, argsX.tiered_input_offset2 + argsX.tiered_transition2,
              ])).filter(x => x >= 0 && x <= maxSpeed).map(x => <circle key={x} cx={`${x / maxSpeed * 100}%`} cy={`${100 - (createEvaluator(argsX)(Math.max(x, 0.001)) - minY) / (maxY - minY) * 100}%`} r="4.5" fill={color} stroke="var(--text-main)" strokeWidth="1" />)}
              {/* Lookup Table Interactive Points */}
              {dataKey === "sensitivity" && (
                <>
                  {argsX.mode === "lookup" && argsX.lookup_table && argsX.lookup_table.map((pt, i) => {
                    const px = (pt.x / maxSpeed) * 100;
                    const sensitivity = argsX.gain ? pt.y / pt.x : pt.y;
                    const py = 100 - ((sensitivity - minY) / (maxY - minY)) * 100;
                    return (
                      <circle
                        key={`lut-x-${i}`}
                        cx={`${px}%`} cy={`${py}%`} r="6"
                        fill="var(--color-x)" stroke="#fff" strokeWidth="2"
                        style={{ cursor: "grab", pointerEvents: "auto" }}
                        onPointerDown={(e) => {
                          e.stopPropagation();
                          setDragging({ axis: "x", index: i });
                          e.currentTarget.setPointerCapture(e.pointerId);
                        }}
                      />
                    );
                  })}
                  {!isSame && argsY.mode === "lookup" && argsY.lookup_table && argsY.lookup_table.map((pt, i) => {
                    const px = (pt.x / maxSpeed) * 100;
                    const sensitivity = argsY.gain ? pt.y / pt.x : pt.y;
                    const py = 100 - ((sensitivity - minY) / (maxY - minY)) * 100;
                    return (
                      <circle
                        key={`lut-y-${i}`}
                        cx={`${px}%`} cy={`${py}%`} r="6"
                        fill="var(--color-y)" stroke="#fff" strokeWidth="2"
                        style={{ cursor: "grab", pointerEvents: "auto" }}
                        onPointerDown={(e) => {
                          e.stopPropagation();
                          setDragging({ axis: "y", index: i });
                          e.currentTarget.setPointerCapture(e.pointerId);
                        }}
                      />
                    );
                  })}
                </>
              )}

              {/* Mouse Tracker Dots */}
              {(() => {
                // Determine which Y value maps to a given X speed on a specific dataset
                const getMappedY = (data: GraphPoint[], key: keyof GraphPoint, speed: number) => {
                  if (data.length === 0 || speed <= data[0].x) return data.length > 0 ? data[0][key] as number : 1;
                  // Find the two points the speed falls between
                  for (let i = 0; i < data.length - 1; i++) {
                    if (speed >= data[i].x && speed <= data[i + 1].x) {
                      // Linear interpolation
                      const t = (speed - data[i].x) / (data[i + 1].x - data[i].x);
                      return (data[i][key] as number) + t * ((data[i + 1][key] as number) - (data[i][key] as number));
                    }
                  }
                  return data[data.length - 1][key] as number; // Fallback to last
                };

                const renderDot = (axis: 'x' | 'y', speed: number, data: GraphPoint[]) => {
                  const yVal = getMappedY(data, dataKey, speed);
                  const px = (Math.min(speed, maxSpeed) / maxSpeed) * 100;
                  const range = maxY - minY;
                  let py = range > 0 ? 100 - ((yVal - minY) / range) * 100 : 100;
                  py = Math.max(0, Math.min(100, py));
                  
                  return (
                    <circle 
                      key={axis}
                      aria-label={`${whole ? 'Combined' : axis.toUpperCase()} preview tracker`}
                      cx={`${px}%`} 
                      cy={`${py}%`} 
                      r="4.5" 
                      fill={whole ? color : `var(--color-${axis})`}
                      stroke="var(--bg-panel)"
                      strokeWidth="2"
                    />
                  );
                };

                // Animate smoothly
                return (
                  <motion.g
                    animate={{ opacity: 1 }}
                    transition={{ type: "tween", ease: "linear", duration: 0.016 }}
                  >
                    {!trackerUnavailable && trackerInputs(whole, mouseSpeed).map(input =>
                      renderDot(input.axis, input.speed, input.axis === 'x' ? dataX : dataY))}
                  </motion.g>
                );
              })()}
            </svg>
            
            {/* X-Axis Title */}
            <div className="graph-input-title" style={{ position: "absolute", bottom: "-2.2rem", left: "50%", transform: "translateX(-50%)", fontSize: "0.65rem", fontWeight: 500, color: "var(--text-muted)", whiteSpace: "nowrap" }}>
              Input Speed (counts/ms)
            </div>
            
          </div>
        </div>
      </motion.div>
    );
  };

  return (
    <div className={`graph-collection ${visibleGraphs.length === 1 ? 'single' : 'multiple'}`}>
      <div className="graph-deck custom-scrollbar">
        <AnimatePresence mode="popLayout">
          {visibleGraphs.includes("sensitivity") && renderGraph("Sensitivity", <Zap size={15} color="var(--color-sensitivity)" />, "sensitivity", "Ratio of Output to Input", "var(--color-sensitivity)")}
          {visibleGraphs.includes("velocity") && renderGraph("Velocity", <Activity size={15} color="var(--color-velocity)" />, "velocity", "Output Velocity (counts/ms)", "var(--color-velocity)")}
          {visibleGraphs.includes("gain") && renderGraph("Gain", <TrendingUp size={15} color="var(--color-gain)" />, "gain", "Slope of Velocity", "var(--color-gain)")}
        </AnimatePresence>
      </div>

      {/* Legend */}
      {!isSame && (
        <div style={{ display: "flex", justifyContent: "center", gap: "1.5rem", marginTop: "0.5rem", fontSize: "0.75rem", color: "var(--text-muted)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <div style={{ width: "16px", height: "3px", backgroundColor: "var(--color-x)" }} /> X Axis
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <div style={{ width: "16px", height: "3px", backgroundColor: "var(--color-y)", borderBottom: "2px dashed var(--bg-app)" }} /> Y Axis
          </div>
        </div>
      )}

    </div>
  );
}
