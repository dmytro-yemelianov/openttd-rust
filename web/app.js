// openttd-rust Frontend Presentation & Canvas Host
(function () {
  const canvas = document.getElementById("gameCanvas");
  const ctx = canvas.getContext("2d");

  let isPlaying = true;
  let tick = 0;
  let zoom = 1.0;
  let panX = 400;
  let panY = 120;
  let isDragging = false;
  let lastMouseX = 0;
  let lastMouseY = 0;

  const MAP_W = 32;
  const MAP_H = 32;
  const TILE_W = 48;
  const TILE_H = 24;

  // Generate deterministic heightmap matching openttd-rust integer generator
  const terrain = [];
  for (let y = 0; y < MAP_H; y++) {
    terrain[y] = [];
    for (let x = 0; x < MAP_W; x++) {
      const distToCenter = Math.hypot(x - 16, y - 16);
      const isWater = distToCenter > 13 || (x < 10 && y < 10);
      const isTown = (x === 16 && y === 16) || (x === 22 && y === 18);
      const isDock = (x === 10 && y === 11) || (x === 18 && y === 24);
      terrain[y][x] = { isWater, isTown, isDock };
    }
  }

  // Ship vehicle state
  const ship = {
    x: 8.0,
    y: 8.0,
    targetX: 18.0,
    targetY: 24.0,
    heading: 0,
  };

  // Convert 2D map coordinate to 2:1 isometric screen coordinate
  function worldToScreen(x, y) {
    const sx = (x - y) * (TILE_W / 2) * zoom + panX;
    const sy = (x + y) * (TILE_H / 2) * zoom + panY;
    return { x: sx, y: sy };
  }

  function drawDiamond(cx, cy, w, h, fillCol, strokeCol) {
    ctx.beginPath();
    ctx.moveTo(cx, cy - h / 2);
    ctx.lineTo(cx + w / 2, cy);
    ctx.lineTo(cx, cy + h / 2);
    ctx.lineTo(cx - w / 2, cy);
    ctx.closePath();
    ctx.fillStyle = fillCol;
    ctx.fill();
    if (strokeCol) {
      ctx.strokeStyle = strokeCol;
      ctx.lineWidth = 1;
      ctx.stroke();
    }
  }

  function render() {
    ctx.fillStyle = "#090d13";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const tw = TILE_W * zoom;
    const th = TILE_H * zoom;

    // Painter's back-to-front depth sorting
    for (let sum = 0; sum < MAP_W + MAP_H; sum++) {
      for (let x = 0; x < MAP_W; x++) {
        const y = sum - x;
        if (y < 0 || y >= MAP_H) continue;

        const tile = terrain[y][x];
        const screen = worldToScreen(x, y);

        // Frustum cull
        if (
          screen.x < -tw ||
          screen.x > canvas.width + tw ||
          screen.y < -th ||
          screen.y > canvas.height + th
        ) {
          continue;
        }

        if (tile.isWater) {
          drawDiamond(screen.x, screen.y, tw, th, "#1e40af", "#1d4ed8");
        } else if (tile.isDock) {
          drawDiamond(screen.x, screen.y, tw, th, "#d97706", "#b45309");
        } else if (tile.isTown) {
          drawDiamond(screen.x, screen.y, tw, th, "#eab308", "#ca8a04");
          // Building block
          ctx.fillStyle = "#f59e0b";
          ctx.fillRect(screen.x - tw / 4, screen.y - th - 8, tw / 2, th + 8);
        } else {
          drawDiamond(screen.x, screen.y, tw, th, "#15803d", "#166534");
        }
      }
    }

    // Render Sub-tick Interpolated Ship
    const shipPos = worldToScreen(ship.x, ship.y);
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(shipPos.x, shipPos.y - 4, 6 * zoom, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#ef4444";
    ctx.font = `${Math.max(10, Math.floor(12 * zoom))}px sans-serif`;
    ctx.fillText("🚢 Ferry", shipPos.x + 8, shipPos.y - 2);
  }

  function tickSim() {
    tick++;
    // Move ship along waterway
    const dx = ship.targetX - ship.x;
    const dy = ship.targetY - ship.y;
    const dist = Math.hypot(dx, dy);

    if (dist > 0.2) {
      ship.x += (dx / dist) * 0.08;
      ship.y += (dy / dist) * 0.08;
    } else {
      // Toggle destination between docks
      if (ship.targetX === 18.0) {
        ship.targetX = 8.0;
        ship.targetY = 8.0;
      } else {
        ship.targetX = 18.0;
        ship.targetY = 24.0;
      }
    }

    // Update telemetry UI
    document.getElementById("tickVal").textContent = tick;
    const trips = Math.min(100, Math.floor(tick / 40));
    document.getElementById("commuteVal").textContent = `${trips} / 100 Trips`;
    const treasury = 100000 + trips * 200;
    document.getElementById("treasuryVal").textContent = `$${treasury.toLocaleString()} / $120,000`;
    document.getElementById("revVal").textContent = `$${(trips * 200).toLocaleString()}`;
    document.getElementById("expVal").textContent = `$${Math.floor(tick * 1.5).toLocaleString()}`;
    document.getElementById("surplusVal").textContent = `+$${Math.max(0, trips * 200 - Math.floor(tick * 1.5)).toLocaleString()}`;
  }

  // Animation Loop (60 FPS render over 30 Hz discrete simulation)
  let lastTickTime = performance.now();
  function loop(time) {
    if (isPlaying && time - lastTickTime >= 33.3) {
      tickSim();
      lastTickTime = time;
    }
    render();
    requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);

  // Mouse drag to pan
  canvas.addEventListener("mousedown", (e) => {
    isDragging = true;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
  });
  window.addEventListener("mouseup", () => (isDragging = false));
  window.addEventListener("mousemove", (e) => {
    if (!isDragging) return;
    panX += e.clientX - lastMouseX;
    panY += e.clientY - lastMouseY;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
  });

  // Wheel to zoom
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    zoom = Math.min(3.0, Math.max(0.4, zoom * factor));
  });

  // Controls
  document.getElementById("btnPlayPause").addEventListener("click", () => {
    isPlaying = !isPlaying;
    document.getElementById("btnPlayPause").textContent = isPlaying ? "Pause" : "Play";
  });
  document.getElementById("btnStep").addEventListener("click", () => {
    tickSim();
    render();
  });
  document.getElementById("btnReset").addEventListener("click", () => {
    tick = 0;
    ship.x = 8.0;
    ship.y = 8.0;
    panX = 400;
    panY = 120;
    zoom = 1.0;
  });
})();
