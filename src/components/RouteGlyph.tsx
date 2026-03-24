import type { GeoRoutePoint, RoutePoint } from "../lib/models";

interface RouteGlyphProps {
  points: RoutePoint[];
  geoPoints?: GeoRoutePoint[];
  title: string;
}

function buildPath(points: RoutePoint[]) {
  return points
    .map((point, index) => {
      const x = 8 + point.x * 84;
      const y = 10 + point.y * 80;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function longitudeToTile(longitude: number, zoom: number) {
  return ((longitude + 180) / 360) * 2 ** zoom;
}

function latitudeToTile(latitude: number, zoom: number) {
  const radians = (latitude * Math.PI) / 180;
  return (
    ((1 - Math.log(Math.tan(radians) + 1 / Math.cos(radians)) / Math.PI) / 2) *
    2 ** zoom
  );
}

function projectGeoPoint(point: GeoRoutePoint, zoom: number) {
  return {
    x: longitudeToTile(point.longitude, zoom) * 256,
    y: latitudeToTile(point.latitude, zoom) * 256,
  };
}

function chooseZoom(points: GeoRoutePoint[], width: number, height: number) {
  for (let zoom = 17; zoom >= 12; zoom -= 1) {
    const projected = points.map((point) => projectGeoPoint(point, zoom));
    const xs = projected.map((point) => point.x);
    const ys = projected.map((point) => point.y);
    const spanX = Math.max(...xs) - Math.min(...xs);
    const spanY = Math.max(...ys) - Math.min(...ys);

    if (spanX <= width - 64 && spanY <= height - 64) {
      return zoom;
    }
  }

  return 12;
}

export function RouteGlyph({ points, geoPoints, title }: RouteGlyphProps) {
  if (!points.length) {
    return null;
  }

  if (geoPoints && geoPoints.length >= 2) {
    const viewportWidth = 640;
    const viewportHeight = 420;
    const padding = 28;
    const zoom = chooseZoom(geoPoints, viewportWidth, viewportHeight);
    const projected = geoPoints.map((point) => projectGeoPoint(point, zoom));
    const xs = projected.map((point) => point.x);
    const ys = projected.map((point) => point.y);
    const minX = Math.min(...xs) - padding;
    const minY = Math.min(...ys) - padding;
    const maxX = Math.max(...xs) + padding;
    const maxY = Math.max(...ys) + padding;
    const width = Math.max(220, maxX - minX);
    const height = Math.max(220, maxY - minY);
    const tileStartX = Math.floor(minX / 256);
    const tileEndX = Math.floor(maxX / 256);
    const tileStartY = Math.floor(minY / 256);
    const tileEndY = Math.floor(maxY / 256);
    const tileColumns: number[] = [];
    const tileRows: number[] = [];

    for (let x = tileStartX; x <= tileEndX; x += 1) {
      tileColumns.push(x);
    }

    for (let y = tileStartY; y <= tileEndY; y += 1) {
      tileRows.push(y);
    }

    const overlayPath = projected
      .map((point, index) => {
        const x = point.x - minX;
        const y = point.y - minY;
        return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
      })
      .join(" ");

    const start = projected[0];
    const finish = projected[projected.length - 1];
    const worldTileCount = 2 ** zoom;

    return (
      <div
        aria-label={`${title} route trace`}
        className="route-glyph route-glyph--map"
        role="img"
        style={{ aspectRatio: `${width} / ${height}` }}
      >
        <div className="route-glyph__tiles">
          {tileRows.map((tileY) =>
            tileColumns.map((tileX) => {
              const wrappedX =
                ((tileX % worldTileCount) + worldTileCount) % worldTileCount;
              const top = tileY * 256 - minY;
              const left = tileX * 256 - minX;
              return (
                <img
                  alt=""
                  className="route-glyph__tile"
                  draggable={false}
                  height={256}
                  key={`${zoom}-${tileX}-${tileY}`}
                  loading="lazy"
                  src={`https://tile.openstreetmap.org/${zoom}/${wrappedX}/${tileY}.png`}
                  width={256}
                  style={{ left, top }}
                />
              );
            }),
          )}
        </div>

        <svg className="route-glyph__overlay" viewBox={`0 0 ${width} ${height}`}>
          <defs>
            <linearGradient id="routeStroke" x1="0" x2="1" y1="0" y2="1">
              <stop offset="0%" stopColor="#ff9e7e" />
              <stop offset="100%" stopColor="#ff5f3c" />
            </linearGradient>
          </defs>

          <rect className="route-glyph__veil" height={height} width={width} x="0" y="0" />

          <path className="route-glyph__path" d={overlayPath} />

          <circle
            className="route-glyph__marker route-glyph__marker--start"
            cx={(start.x - minX).toFixed(2)}
            cy={(start.y - minY).toFixed(2)}
            r="6"
          />
          <circle
            className="route-glyph__marker route-glyph__marker--finish"
            cx={(finish.x - minX).toFixed(2)}
            cy={(finish.y - minY).toFixed(2)}
            r="7"
          />
        </svg>

        <p className="route-glyph__attribution">Map data © OpenStreetMap contributors</p>
      </div>
    );
  }

  const start = points[0];
  const finish = points[points.length - 1];

  return (
    <svg
      aria-label={`${title} route trace`}
      className="route-glyph"
      role="img"
      viewBox="0 0 100 100"
    >
      <defs>
        <linearGradient id="routeStroke" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0%" stopColor="#ff9e7e" />
          <stop offset="100%" stopColor="#ff5f3c" />
        </linearGradient>
      </defs>

      {[20, 50, 80].map((position) => (
        <line
          key={`h-${position}`}
          className="route-glyph__grid"
          x1="8"
          x2="92"
          y1={position}
          y2={position}
        />
      ))}

      {[20, 50, 80].map((position) => (
        <line
          key={`v-${position}`}
          className="route-glyph__grid"
          x1={position}
          x2={position}
          y1="10"
          y2="90"
        />
      ))}

      <path className="route-glyph__path" d={buildPath(points)} />

      <circle
        className="route-glyph__marker route-glyph__marker--start"
        cx={(8 + start.x * 84).toFixed(2)}
        cy={(10 + start.y * 80).toFixed(2)}
        r="2.8"
      />
      <circle
        className="route-glyph__marker route-glyph__marker--finish"
        cx={(8 + finish.x * 84).toFixed(2)}
        cy={(10 + finish.y * 80).toFixed(2)}
        r="3.2"
      />
    </svg>
  );
}
