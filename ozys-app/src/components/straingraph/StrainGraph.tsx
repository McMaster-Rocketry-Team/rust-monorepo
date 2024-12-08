import { useTabAtom } from '../../workspace/useTabAtom';
import { useOzysDevicesManager } from '../../device/OzysDevicesManager'
import { Dispatch, SetStateAction, useEffect, useRef, useState } from 'react'
import { RealtimeReadingsPlayer } from '../../database/RealtimeReadingsPlayer'
import { Remote } from 'comlink'
import { useWillUnmount } from 'rooks'
import { Mutex } from 'async-mutex'
import { autorun } from 'mobx'

const usePlayers = (
    selectedChannels: string[],
    setSelectedChannels: Dispatch<SetStateAction<string[]>>,
) => {
    const devicesManager = useOzysDevicesManager()
    const playersRef = useRef<Map<string, Remote<RealtimeReadingsPlayer>>>()
    const playersMutexRef = useRef<Mutex>()

    if (!playersRef.current) {
        playersRef.current = new Map()
        playersMutexRef.current = new Mutex()
    }

    // remove selected channels that are not in allChannels
    // autorun is needed here because devicesManager.activeChannels
    // is not a react state (but a mobx computed property)
    useEffect(
        () =>
            autorun(() => {
                const activeChannelIds = devicesManager.activeChannels.map(
                    (channel) => channel.channel.id,
                )
                setSelectedChannels((old) =>
                    old.filter((channelId) => activeChannelIds.includes(channelId)),
                )
            }),
        [],
    )

    useEffect(() => {
        playersMutexRef.current!.runExclusive(async () => {
            // create new players for selected channels
            for (const selectedChannel of selectedChannels) {
                if (!playersRef.current!.has(selectedChannel)) {
                    const player = await devicesManager.createRealtimeReadingsPlayer(
                        selectedChannel,
                        60, // TODO
                        0,
                    )
                    playersRef.current!.set(selectedChannel, player)
                }
            }

            // dispose players for unselected channels
            for (const [channelId, player] of playersRef.current!.entries()) {
                if (!selectedChannels.includes(channelId)) {
                    player.dispose()
                    playersRef.current!.delete(channelId)
                }
            }
        })
    }, [selectedChannels])

    useWillUnmount(() => {
        playersMutexRef.current!.runExclusive(async () => {
            for (const player of playersRef.current!.values()) {
                player.dispose()
            }
        })
    })

    return playersRef.current
}

export const StrainGraph = () => {
    const [text, setText] = useTabAtom('text', 'hello');
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [time, setTime] = useState(0);
    const [hoverInfo, setHoverInfo] = useState<{
        x: number;
        dataIndex: number | null;
    } | null>(null);
    const [data, setData] = useState<{ time: number; value1: number; value2: number }[]>([]);
    const [isMenuOpen, setIsMenuOpen] = useState(false); // Popup menu state
    const [dataConfig, setDataConfig] = useState([
        { key: 'value1', color: 'blue', interval: 100 },
        { key: 'value2', color: 'orange', interval: 100 },
    ]);

    // Generate dynamic data
    useEffect(() => {
        const interval = setInterval(() => {
            setTime((prev) => prev + 1);
            setData((prev) => {
                const newDataPoint = {
                    time: time,
                    value1: Math.sin(time / 50) * 100,
                    value2: Math.cos(time / 50) * 80,
                };
                return [...prev.slice(-100), newDataPoint];
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

        const resizeCanvas = () => {
            if (canvas.parentElement) {
                canvas.width = canvas.parentElement.clientWidth;
                canvas.height = canvas.parentElement.clientHeight;
            }
        };
        resizeCanvas(); // Initial resize

        const handleResize = () => resizeCanvas();
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
        const timeScale = canvas.width / 100;
        const valueScale = canvas.height / 200;

        // Draw sensor lines
        dataConfig.forEach(({ key, color }) => {
            ctx.beginPath();
            ctx.strokeStyle = color;
            data.forEach((point, index) => {
                const x = index * timeScale;
                const y = canvas.height / 2 - (point[key as keyof typeof point] as number) * valueScale;
                if (index === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            });
            ctx.stroke();
        });

        // Draw hover line and values
        if (hoverInfo && hoverInfo.dataIndex !== null) {
            const { x, dataIndex } = hoverInfo;
            const hoveredPoint = data[dataIndex];

            // Draw vertical hover line
            ctx.beginPath();
            ctx.strokeStyle = 'gray';
            ctx.setLineDash([5, 5]);
            ctx.moveTo(x, 0);
            ctx.lineTo(x, canvas.height);
            ctx.stroke();
            ctx.setLineDash([]);

            // Draw hover data values
            ctx.font = '12px Arial';
            ctx.fillStyle = 'black';
            ctx.textAlign = 'left';

            const textX = x + 10;
            const textYStart = 20;
            ctx.fillText(`Time: ${hoveredPoint.time}ms`, textX, textYStart);
            dataConfig.forEach(({ key, color }, idx) => {
                ctx.fillText(`${key}: ${(hoveredPoint[key as keyof typeof hoveredPoint] as number).toFixed(2)}`, textX, textYStart + 15 * (idx + 1));
            });
        }

        return () => window.removeEventListener('resize', handleResize);
    }, [data, hoverInfo, dataConfig]);

    // Handle mouse hover
    const handleMouseMove = (event: React.MouseEvent) => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const rect = canvas.getBoundingClientRect();
        const mouseX = event.clientX - rect.left;

        const timeScale = canvas.width / 100;
        const dataIndex = Math.floor(mouseX / timeScale);

        if (dataIndex >= 0 && dataIndex < data.length) {
            setHoverInfo({ x: mouseX, dataIndex });
        } else {
            setHoverInfo(null);
        }
    };

    const handleMouseLeave = () => {
        setHoverInfo(null);
    };

    const toggleMenu = () => setIsMenuOpen((prev) => !prev);

    const updateConfig = (index: number, newConfig: Partial<{ color: string; interval: number }>) => {
        setDataConfig((prev) =>
            prev.map((config, i) => (i === index ? { ...config, ...newConfig } : config))
        );
    };

    return (
        <div style={{ width: '100%', height: '100%', position: 'relative' }}>
            {/* Toggle Button */}
            <button
                onClick={toggleMenu}
                style={{
                    position: 'absolute',
                    top: '10px',
                    left: '10px',
                    zIndex: 10,
                }}
            >
                {isMenuOpen ? 'Close Menu' : 'Open Menu'}
            </button>

            {/* Popup Menu */}
            {isMenuOpen && (
                <div
                    style={{
                        position: 'absolute',
                        top: '40px',
                        left: '10px',
                        padding: '10px',
                        border: '1px solid black',
                        backgroundColor: 'white',
                        zIndex: 10,
                    }}
                >
                    <h4>Data Configurations</h4>
                    {dataConfig.map((config, index) => (
                        <div key={index} style={{ marginBottom: '10px' }}>
                            <label>
                                Key: {config.key}
                                <br />
                                Color:
                                <input
                                    type="color"
                                    value={config.color}
                                    onChange={(e) =>
                                        updateConfig(index, { color: e.target.value })
                                    }
                                />
                            </label>
                            <br />
                            <label>
                                Interval (ms):
                                <input
                                    type="number"
                                    value={config.interval}
                                    onChange={(e) =>
                                        updateConfig(index, { interval: parseInt(e.target.value, 10) || 0 })
                                    }
                                />
                            </label>
                        </div>
                    ))}
                </div>
            )}

            {/* Graph Canvas */}
            <canvas
                ref={canvasRef}
                style={{
                    display: 'block',
                    width: '100%',
                    height: '100%',
                    padding: '10px',
                }}
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
            />
        </div>
    );
};
