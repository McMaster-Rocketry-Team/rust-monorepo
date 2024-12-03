import React, { useEffect, useRef, useState } from 'react';
import { useTabAtom } from '../../workspace/useTabAtom';

export const StrainGraph = () => {
  const [text, setText] = useTabAtom('text', 'hello');
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [time, setTime] = useState(0);
  const [hoverInfo, setHoverInfo] = useState<{
    time: number;
    dataIndex: number | null;
  } | null>(null);
  const [data, setData] = useState<{ time: number; value1: number; value2: number }[]>([]);


  // Generate dynamic data
  useEffect(() => {
    const interval = setInterval(() => {
      setTime((prev) => prev + 1);
      setData((prev) => {
        const newDataPoint = {
          time: time,
          value1: Math.sin(time / 50) * 100, // Example sine wave for sensor 1
          value2: Math.cos(time / 50) * 80, // Example cosine wave for sensor 2
        };
        return [...prev.slice(-100), newDataPoint]; // Keep only the latest 100 points
      });
    }, 100); // Update every 100ms

    return () => clearInterval(interval);
  }, [time]);

  // Render graph on canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Resize the canvas to fill its parent
    const resizeCanvas = () => {
      if (canvas.parentElement) {
        canvas.width = canvas.parentElement.clientWidth;
        canvas.height = canvas.parentElement.clientHeight;
      }
    };
    resizeCanvas(); // Initial resize

    // Redraw the graph on window resize
    const handleResize = () => {
      resizeCanvas();
    };
    window.addEventListener('resize', handleResize);

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw grid lines
    ctx.strokeStyle = '#e0e0e0';
    for (let x = 0; x <= canvas.width; x += 50) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvas.height);
      ctx.stroke();
    }
    for (let y = 0; y <= canvas.height; y += 50) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(canvas.width, y);
      ctx.stroke();
    }

    // Define scales
    const timeScale = canvas.width / 100; // Scale time to canvas width
    const valueScale = canvas.height / 200; // Scale values to canvas height (-100 to 100)

    // Draw sensor lines
    const drawLine = (key: 'value1' | 'value2', color: string) => {
      ctx.beginPath();
      ctx.strokeStyle = color;
      data.forEach((point, index) => {
        const x = index * timeScale;
        const y = canvas.height / 2 - point[key] * valueScale;
        if (index === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    };

    drawLine('value1', 'blue'); // Sensor 1 in blue
    drawLine('value2', 'orange'); // Sensor 2 in orange

    // Draw time labels on x-axis
    ctx.font = '12px Arial';
    ctx.fillStyle = 'black';
    ctx.textAlign = 'center';
      
    const labelInterval = 10; // Label every 10 data points
    for (let i = 0; i < data.length; i += labelInterval) {
      const x = i * timeScale;
      const timeLabel = data[i].time; // Get time value
      ctx.fillText(`${timeLabel}ms`, x, canvas.height - 5); // Draw time label near the bottom
    }

    return () => window.removeEventListener('resize', handleResize);
  }, [data]);

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative' }}>
      <canvas
        ref={canvasRef}
        style={{
          display: 'block',
          width: '100%',
          height: '100%',
          padding: '10px'
        }}
      />
    </div>
  );
};
