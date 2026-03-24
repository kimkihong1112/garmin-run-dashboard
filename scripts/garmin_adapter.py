#!/usr/bin/env python3

from __future__ import annotations

import base64
import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import garth
import requests
from garminconnect import (
    Garmin,
    GarminConnectAuthenticationError,
    GarminConnectConnectionError,
)


def read_payload() -> dict[str, Any]:
    raw = sys.stdin.read().strip()
    if not raw:
        return {}
    return json.loads(raw)


def write_result(payload: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(payload))


def now_iso() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def ts_to_iso(timestamp: int | None) -> str:
    if not timestamp:
        return now_iso()
    return datetime.fromtimestamp(timestamp, UTC).isoformat().replace("+00:00", "Z")


def build_public_session(client: Garmin, email: str) -> dict[str, Any]:
    oauth2 = client.garth.oauth2_token
    token_last_four = "----"

    if oauth2:
        refresh_token = getattr(oauth2, "refresh_token", "")
        token_last_four = str(refresh_token)[-4:].upper().rjust(4, "0")

    return {
        "athleteName": client.display_name or client.get_full_name() or "Garmin Runner",
        "fullName": client.get_full_name(),
        "accountEmail": email,
        "issuedAt": now_iso(),
        "expiresAt": ts_to_iso(
            getattr(oauth2, "refresh_token_expires_at", None)
            or getattr(oauth2, "expires_at", None)
        ),
        "tokenLastFour": token_last_four,
        "unitSystem": client.get_unit_system(),
    }


def serialize_challenge_state(client_state: dict[str, Any]) -> str:
    client = client_state["client"]
    serialized = {
        "domain": client.domain,
        "signinParams": client_state["signin_params"],
        "cookies": requests.utils.dict_from_cookiejar(client.sess.cookies),
        "lastResponseText": client.last_resp.text,
        "lastResponseUrl": client.last_resp.url,
    }
    return base64.b64encode(json.dumps(serialized).encode("utf-8")).decode("utf-8")


def deserialize_challenge_state(encoded: str) -> dict[str, Any]:
    return json.loads(base64.b64decode(encoded.encode("utf-8")).decode("utf-8"))


def restore_mfa_client(encoded_state: str) -> tuple[garth.Client, dict[str, Any]]:
    challenge_state = deserialize_challenge_state(encoded_state)
    client = garth.Client(domain=challenge_state["domain"])
    client.sess.cookies = requests.utils.cookiejar_from_dict(challenge_state["cookies"])

    response = requests.Response()
    response.status_code = 200
    response.url = challenge_state["lastResponseUrl"]
    response._content = challenge_state["lastResponseText"].encode("utf-8")
    response.encoding = "utf-8"
    client.last_resp = response

    return client, challenge_state["signinParams"]


def complete_resume_login(email: str, encoded_state: str, mfa_code: str) -> dict[str, Any]:
    if not mfa_code or not mfa_code.isdigit() or len(mfa_code) != 6:
        raise GarminConnectAuthenticationError(
            "Verification codes must contain exactly 6 digits."
        )

    client, signin_params = restore_mfa_client(encoded_state)
    csrf_token = garth.sso.get_csrf_token(client.last_resp.text)
    client.post(
        "sso",
        "/sso/verifyMFA/loginEnterMfaCode",
        params=signin_params,
        referrer=True,
        data={
            "mfa-code": mfa_code,
            "embed": "true",
            "_csrf": csrf_token,
            "fromPage": "setupEnterMfaCode",
        },
    )

    title = garth.sso.get_title(client.last_resp.text)
    if title != "Success":
        raise GarminConnectAuthenticationError(
            "Garmin did not accept the verification code."
        )

    garth.sso._complete_login(client)

    garmin_client = Garmin()
    garmin_client.garth = client

    profile = garmin_client.garth.connectapi("/userprofile-service/userprofile/profile")
    if profile and isinstance(profile, dict):
        garmin_client.display_name = profile.get("displayName")
        garmin_client.full_name = profile.get("fullName")

    settings = garmin_client.garth.connectapi(
        garmin_client.garmin_connect_user_settings_url
    )
    if settings and isinstance(settings, dict) and "userData" in settings:
        garmin_client.unit_system = settings["userData"].get("measurementSystem")

    return {
        "status": "authenticated",
        "session": build_public_session(garmin_client, email),
        "tokenStore": garmin_client.garth.dumps(),
    }


def authenticate(payload: dict[str, Any]) -> dict[str, Any]:
    email = str(payload.get("email", "")).strip().lower()
    password = str(payload.get("password", ""))

    if not email or not password:
        raise GarminConnectAuthenticationError(
            "Enter both your Garmin email and password."
        )

    client = Garmin(email, password, return_on_mfa=True)
    result1, result2 = client.login()

    if result1 == "needs_mfa":
        return {
            "status": "mfa_required",
            "message": "Garmin requested a six-digit verification code.",
            "challengeState": serialize_challenge_state(result2),
            "accountEmail": email,
        }

    return {
        "status": "authenticated",
        "session": build_public_session(client, email),
        "tokenStore": client.garth.dumps(),
    }


def pick_number(*values: Any) -> float | None:
    for value in values:
        if value is None:
            continue
        if isinstance(value, (int, float)):
            return float(value)
        if isinstance(value, str):
            try:
                return float(value)
            except ValueError:
                continue
    return None


def pick_nested(mapping: dict[str, Any], *paths: str) -> Any:
    for path in paths:
        current: Any = mapping
        for segment in path.split("."):
            if not isinstance(current, dict) or segment not in current:
                current = None
                break
            current = current[segment]
        if current is not None:
            return current
    return None


def normalize_activity(
    summary: dict[str, Any],
    details: dict[str, Any] | None,
    raw_file_path: str,
) -> dict[str, Any]:
    activity_id = str(summary.get("activityId"))
    distance_meters = pick_number(
        summary.get("distance"),
        pick_nested(details or {}, "summaryDTO.distance"),
    )
    duration_seconds = pick_number(
        summary.get("duration"),
        summary.get("movingDuration"),
        pick_nested(details or {}, "summaryDTO.duration"),
    )
    average_speed = pick_number(
        summary.get("averageSpeed"),
        pick_nested(details or {}, "summaryDTO.averageSpeed"),
    )
    average_pace_seconds_per_km = None

    if average_speed and average_speed > 0:
        average_pace_seconds_per_km = 1000.0 / average_speed
    elif distance_meters and duration_seconds and distance_meters > 0:
        average_pace_seconds_per_km = duration_seconds / (distance_meters / 1000.0)

    activity_name = (
        summary.get("activityName")
        or pick_nested(details or {}, "activityName")
        or "Running activity"
    )
    activity_type = (
        pick_nested(summary, "activityType.typeKey")
        or pick_nested(details or {}, "activityTypeDTO.typeKey")
        or "running"
    )
    source_created_at = (
        summary.get("startTimeGMT")
        or summary.get("startTimeGmt")
        or summary.get("startTimeLocal")
        or now_iso()
    )
    time_zone = (
        pick_nested(summary, "timeZoneUnitDTO.unitKey")
        or pick_nested(details or {}, "timeZoneUnitDTO.unitKey")
        or "UTC"
    )

    normalized = {
        "activityId": activity_id,
        "activityName": activity_name,
        "activityType": activity_type,
        "sourceCreatedAt": source_created_at,
        "startTimeLocal": summary.get("startTimeLocal") or source_created_at,
        "startTimeGmt": summary.get("startTimeGMT")
        or summary.get("startTimeGmt")
        or source_created_at,
        "timeZone": time_zone,
        "distanceMeters": distance_meters,
        "durationSeconds": duration_seconds,
        "averagePaceSecondsPerKm": average_pace_seconds_per_km,
        "averageHeartRate": pick_number(
            summary.get("averageHR"),
            pick_nested(details or {}, "summaryDTO.averageHR"),
        ),
        "maxHeartRate": pick_number(
            summary.get("maxHR"),
            pick_nested(details or {}, "summaryDTO.maxHR"),
        ),
        "elevationGainMeters": pick_number(
            summary.get("elevationGain"),
            pick_nested(details or {}, "summaryDTO.elevationGain"),
        ),
        "trainingLoad": pick_number(
            summary.get("activityTrainingLoad"),
            pick_nested(details or {}, "summaryDTO.activityTrainingLoad"),
        ),
        "rawFilePath": raw_file_path,
        "summaryJson": summary,
        "normalizedJson": {
            "rawFilePath": raw_file_path,
            "summaryKeys": sorted(summary.keys()),
            "detailKeys": sorted((details or {}).keys()),
        },
    }

    return normalized


def sync_running(payload: dict[str, Any]) -> dict[str, Any]:
    token_store = str(payload.get("tokenStore", "")).strip()
    raw_data_dir = Path(str(payload.get("rawDataDir", "")).strip())
    max_activities = int(payload.get("maxActivities", 120))
    account_email = str(payload.get("accountEmail", "")).strip().lower()

    if not token_store:
        raise GarminConnectAuthenticationError(
            "A stored Garmin token payload is required before syncing."
        )

    if not raw_data_dir:
        raise GarminConnectConnectionError("A raw data directory is required.")

    activities_dir = raw_data_dir / "activities"
    manifests_dir = raw_data_dir / "manifests"
    activities_dir.mkdir(parents=True, exist_ok=True)
    manifests_dir.mkdir(parents=True, exist_ok=True)

    client = Garmin()
    client.login(tokenstore=token_store)

    normalized_activities: list[dict[str, Any]] = []
    warnings: list[str] = []
    page_start = 0
    page_limit = 20

    while len(normalized_activities) < max_activities:
        page = client.get_activities(
            start=page_start,
            limit=min(page_limit, max_activities - len(normalized_activities)),
            activitytype="running",
        )
        if not page:
            break

        for summary in page:
            activity_id = str(summary.get("activityId", "")).strip()
            if not activity_id:
                continue

            details = None
            splits = None

            try:
                details = client.get_activity_details(activity_id)
            except Exception as error:
                warnings.append(f"Activity {activity_id}: failed to fetch details ({error})")

            try:
                splits = client.get_activity_splits(activity_id)
            except Exception as error:
                warnings.append(f"Activity {activity_id}: failed to fetch splits ({error})")

            raw_payload = {
                "syncedAt": now_iso(),
                "summary": summary,
                "details": details,
                "splits": splits,
            }
            raw_file_path = activities_dir / f"{activity_id}.json"
            raw_file_path.write_text(
                json.dumps(raw_payload, ensure_ascii=False, separators=(",", ":")),
                encoding="utf-8",
            )

            normalized_activities.append(
                normalize_activity(summary, details, str(raw_file_path))
            )

            if len(normalized_activities) >= max_activities:
                break

        page_start += page_limit

    manifest = {
        "syncedAt": now_iso(),
        "activityCount": len(normalized_activities),
        "activityIds": [item["activityId"] for item in normalized_activities],
        "warnings": warnings,
    }
    manifest_path = manifests_dir / f"sync-{datetime.now(UTC).strftime('%Y%m%dT%H%M%SZ')}.json"
    manifest_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    session = build_public_session(client, account_email or "unknown@garmin.local")

    return {
        "status": "synchronized",
        "session": session,
        "tokenStore": client.garth.dumps(),
        "activities": normalized_activities,
        "manifestPath": str(manifest_path),
        "warnings": warnings,
    }


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("Usage: garmin_adapter.py <authenticate|resume-mfa|sync-running>")

    command = sys.argv[1]
    payload = read_payload()

    try:
        if command == "authenticate":
            write_result(authenticate(payload))
            return

        if command == "resume-mfa":
            write_result(
                complete_resume_login(
                    email=str(payload.get("email", "")).strip().lower(),
                    encoded_state=str(payload.get("challengeState", "")),
                    mfa_code=str(payload.get("mfaCode", "")).strip(),
                )
            )
            return

        if command == "sync-running":
            write_result(sync_running(payload))
            return

        raise SystemExit(f"Unknown command: {command}")
    except (
        GarminConnectAuthenticationError,
        GarminConnectConnectionError,
        garth.exc.GarthException,
        ValueError,
    ) as error:
        sys.stderr.write(str(error))
        raise SystemExit(1) from error


if __name__ == "__main__":
    main()
