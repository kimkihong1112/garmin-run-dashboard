#!/usr/bin/env python3

from __future__ import annotations

import base64
import html
import json
import re
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs

import requests
from requests import Response, Session
from requests.adapters import HTTPAdapter, Retry
from requests_oauthlib import OAuth1Session
from garminconnect import (
    Garmin,
    GarminConnectAuthenticationError,
    GarminConnectConnectionError,
)

GARMIN_DOMAIN = "garmin.com"
GARMIN_SSO_URL = f"https://sso.{GARMIN_DOMAIN}/sso"
GARMIN_SSO_EMBED_URL = f"{GARMIN_SSO_URL}/embed"
GARMIN_SSO_EMBED_PARAMS = {
    "id": "gauth-widget",
    "embedWidget": "true",
    "gauthHost": GARMIN_SSO_URL,
}
GARMIN_SIGNIN_PARAMS = {
    **GARMIN_SSO_EMBED_PARAMS,
    **{
        "gauthHost": GARMIN_SSO_EMBED_URL,
        "service": GARMIN_SSO_EMBED_URL,
        "source": GARMIN_SSO_EMBED_URL,
        "redirectAfterAccountLoginUrl": GARMIN_SSO_EMBED_URL,
        "redirectAfterAccountCreationUrl": GARMIN_SSO_EMBED_URL,
    },
}
GARMIN_MOBILE_USER_AGENT = "com.garmin.android.apps.connectmobile"
GARMIN_APP_USER_AGENT = "GCM-iOS-5.19.1.2"
OAUTH_CONSUMER_URL = "https://thegarth.s3.amazonaws.com/oauth_consumer.json"
CSRF_RE = re.compile(r'name="_csrf"\s+value="(.+?)"')
TITLE_RE = re.compile(r"<title>(.+?)</title>")
TICKET_RE = re.compile(r'embed\?ticket=([^"]+)"')
HTML_TAG_RE = re.compile(r"<[^>]+>")
AUTH_REQUEST_TIMEOUT_SECONDS = 12
AUTH_RETRY_COUNT = 1
SYNC_REQUEST_TIMEOUT_SECONDS = 12
SYNC_RETRY_COUNT = 2


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


def configure_retry_session(session: Session, retries: int, backoff_factor: float) -> None:
    adapter = HTTPAdapter(
        max_retries=Retry(
            total=retries,
            status_forcelist=(408, 500, 502, 503, 504),
            backoff_factor=backoff_factor,
        )
    )
    session.mount("https://", adapter)


def build_auth_session() -> Session:
    session = requests.Session()
    session.headers.update({"User-Agent": GARMIN_APP_USER_AGENT})
    configure_retry_session(session, retries=AUTH_RETRY_COUNT, backoff_factor=0.25)
    return session


def get_oauth_consumer(session: Session) -> dict[str, Any]:
    try:
        response = session.get(OAUTH_CONSUMER_URL, timeout=AUTH_REQUEST_TIMEOUT_SECONDS)
        response.raise_for_status()
    except requests.RequestException as error:
        raise GarminConnectConnectionError(
            f"Failed to load the Garmin OAuth consumer settings: {error}"
        ) from error
    return response.json()


def get_csrf_token(html_text: str) -> str:
    match = CSRF_RE.search(html_text)
    if not match:
        raise GarminConnectAuthenticationError(
            "Garmin did not return a login form token. Please try again."
        )
    return match.group(1)


def get_title(html_text: str) -> str:
    match = TITLE_RE.search(html_text)
    if not match:
        raise GarminConnectAuthenticationError(
            "Garmin returned an unexpected login response. Please try again."
        )
    return match.group(1)


def strip_html(raw_text: str) -> str:
    cleaned = HTML_TAG_RE.sub(" ", raw_text)
    return " ".join(html.unescape(cleaned).split())


def extract_page_error(html_text: str) -> str | None:
    markers = [
        "error-message",
        "alert-danger",
        "message--error",
        "form-error",
        "gauth-form-error",
    ]

    lowercase_html = html_text.lower()
    for marker in markers:
        marker_index = lowercase_html.find(marker)
        if marker_index == -1:
            continue

        snippet = html_text[max(0, marker_index - 120) : marker_index + 420]
        message = strip_html(snippet)
        if message:
            return message

    return None


def map_http_error(error: requests.HTTPError) -> GarminConnectConnectionError:
    status_code = getattr(error.response, "status_code", None)

    if status_code == 429:
        return GarminConnectConnectionError(
            "Garmin temporarily rate-limited the sign-in request. Wait a moment, then try again."
        )

    if status_code in {401, 403}:
        return GarminConnectConnectionError(
            "Garmin rejected the sign-in request. Double-check the email and password, then try again."
        )

    return GarminConnectConnectionError(f"Garmin request failed: {error}")


def set_expirations(token: dict[str, Any]) -> dict[str, Any]:
    token["expires_at"] = int(time.time() + int(token["expires_in"]))
    token["refresh_token_expires_at"] = int(
        time.time() + int(token["refresh_token_expires_in"])
    )
    return token


def build_token_store(
    oauth1_token: dict[str, Any],
    oauth2_token: dict[str, Any],
) -> str:
    payload = json.dumps([oauth1_token, oauth2_token])
    return base64.b64encode(payload.encode("utf-8")).decode("utf-8")


def fetch_oauth1_token(ticket: str, session: Session) -> dict[str, Any]:
    consumer = get_oauth_consumer(session)
    oauth_session = OAuth1Session(
        consumer["consumer_key"],
        consumer["consumer_secret"],
    )
    oauth_session.headers.update({"User-Agent": GARMIN_MOBILE_USER_AGENT})
    oauth_session.mount("https://", session.adapters["https://"])

    login_url = GARMIN_SSO_EMBED_URL
    url = (
        f"https://connectapi.{GARMIN_DOMAIN}/oauth-service/oauth/preauthorized"
        f"?ticket={ticket}&login-url={login_url}&accepts-mfa-tokens=true"
    )
    try:
        response = oauth_session.get(url, timeout=AUTH_REQUEST_TIMEOUT_SECONDS)
        response.raise_for_status()
    except requests.RequestException as error:
        raise GarminConnectConnectionError(
            f"Failed to exchange the Garmin login ticket for an OAuth1 token: {error}"
        ) from error

    token = {key: values[0] for key, values in parse_qs(response.text).items()}
    token["domain"] = GARMIN_DOMAIN
    return token


def exchange_oauth2_token(oauth1_token: dict[str, Any], session: Session) -> dict[str, Any]:
    consumer = get_oauth_consumer(session)
    oauth_session = OAuth1Session(
        consumer["consumer_key"],
        consumer["consumer_secret"],
        resource_owner_key=oauth1_token["oauth_token"],
        resource_owner_secret=oauth1_token["oauth_token_secret"],
    )
    oauth_session.headers.update({"User-Agent": GARMIN_MOBILE_USER_AGENT})
    oauth_session.mount("https://", session.adapters["https://"])

    data: dict[str, Any] = {}
    if oauth1_token.get("mfa_token"):
        data["mfa_token"] = oauth1_token["mfa_token"]

    try:
        response = oauth_session.post(
            f"https://connectapi.{GARMIN_DOMAIN}/oauth-service/oauth/exchange/user/2.0",
            headers={"Content-Type": "application/x-www-form-urlencoded"},
            data=data,
            timeout=AUTH_REQUEST_TIMEOUT_SECONDS,
        )
        response.raise_for_status()
    except requests.RequestException as error:
        raise GarminConnectConnectionError(
            f"Failed to exchange the Garmin OAuth1 token for OAuth2 access: {error}"
        ) from error
    token = response.json()
    return set_expirations(token)


def restore_session_from_tokens(email: str, token_store: str) -> dict[str, Any]:
    client = Garmin()
    client.login(tokenstore=token_store)
    return {
        "status": "authenticated",
        "session": build_public_session(client, email),
        "tokenStore": token_store,
    }


def request_page(
    session: Session,
    method: str,
    url: str,
    *,
    previous_response: Response | None = None,
    **kwargs: Any,
) -> Response:
    headers = kwargs.pop("headers", {})
    if previous_response is not None:
        headers = {**headers, "referer": previous_response.url}

    try:
        response = session.request(
            method,
            url,
            headers=headers,
            timeout=AUTH_REQUEST_TIMEOUT_SECONDS,
            **kwargs,
        )
        response.raise_for_status()
    except requests.HTTPError as error:
        raise map_http_error(error) from error
    except requests.RequestException as error:
        raise GarminConnectConnectionError(
            f"Garmin sign-in request could not be completed: {error}"
        ) from error

    return response


def serialize_challenge_state(challenge_state: dict[str, Any]) -> str:
    serialized = {
        "domain": GARMIN_DOMAIN,
        "signinParams": challenge_state["signinParams"],
        "cookies": requests.utils.dict_from_cookiejar(challenge_state["session"].cookies),
        "lastResponseText": challenge_state["lastResponseText"],
        "lastResponseUrl": challenge_state["lastResponseUrl"],
    }
    return base64.b64encode(json.dumps(serialized).encode("utf-8")).decode("utf-8")


def deserialize_challenge_state(encoded: str) -> dict[str, Any]:
    return json.loads(base64.b64decode(encoded.encode("utf-8")).decode("utf-8"))


def restore_mfa_challenge(encoded_state: str) -> dict[str, Any]:
    challenge_state = deserialize_challenge_state(encoded_state)
    session = build_auth_session()
    session.cookies = requests.utils.cookiejar_from_dict(challenge_state["cookies"])

    response = Response()
    response.status_code = 200
    response.url = challenge_state["lastResponseUrl"]
    response._content = challenge_state["lastResponseText"].encode("utf-8")
    response.encoding = "utf-8"

    return {
        "session": session,
        "response": response,
        "signinParams": challenge_state["signinParams"],
    }


def complete_login(session: Session, final_response: Response) -> str:
    match = TICKET_RE.search(final_response.text)
    if not match:
        raise GarminConnectAuthenticationError(
            "Garmin finished sign-in without returning an OAuth ticket."
        )

    ticket = match.group(1)
    oauth1_token = fetch_oauth1_token(ticket, session)
    oauth2_token = exchange_oauth2_token(oauth1_token, session)
    return build_token_store(oauth1_token, oauth2_token)


def complete_resume_login(email: str, encoded_state: str, mfa_code: str) -> dict[str, Any]:
    if not mfa_code or not mfa_code.isdigit() or len(mfa_code) != 6:
        raise GarminConnectAuthenticationError(
            "Verification codes must contain exactly 6 digits."
        )

    challenge_state = restore_mfa_challenge(encoded_state)
    session = challenge_state["session"]
    previous_response = challenge_state["response"]
    signin_params = challenge_state["signinParams"]
    csrf_token = get_csrf_token(previous_response.text)
    response = request_page(
        session,
        "POST",
        f"{GARMIN_SSO_URL}/verifyMFA/loginEnterMfaCode",
        params=signin_params,
        previous_response=previous_response,
        data={
            "mfa-code": mfa_code,
            "embed": "true",
            "_csrf": csrf_token,
            "fromPage": "setupEnterMfaCode",
        },
    )

    title = get_title(response.text)
    if title != "Success":
        page_error = extract_page_error(response.text)
        raise GarminConnectAuthenticationError(
            page_error or "Garmin did not accept the verification code."
        )

    token_store = complete_login(session, response)
    return restore_session_from_tokens(email, token_store)


def authenticate(payload: dict[str, Any]) -> dict[str, Any]:
    email = str(payload.get("email", "")).strip().lower()
    password = str(payload.get("password", ""))

    if not email or not password:
        raise GarminConnectAuthenticationError(
            "Enter both your Garmin email and password."
        )

    session = build_auth_session()

    try:
        embed_response = request_page(
            session,
            "GET",
            GARMIN_SSO_EMBED_URL,
            params=GARMIN_SSO_EMBED_PARAMS,
        )
        signin_page = request_page(
            session,
            "GET",
            f"{GARMIN_SSO_URL}/signin",
            params=GARMIN_SIGNIN_PARAMS,
            previous_response=embed_response,
        )
        csrf_token = get_csrf_token(signin_page.text)
        signin_response = request_page(
            session,
            "POST",
            f"{GARMIN_SSO_URL}/signin",
            params=GARMIN_SIGNIN_PARAMS,
            previous_response=signin_page,
            data={
                "username": email,
                "password": password,
                "embed": "true",
                "_csrf": csrf_token,
            },
        )
    except requests.RequestException as error:
        raise GarminConnectConnectionError(
            f"Garmin sign-in request could not be completed: {error}"
        ) from error

    title = get_title(signin_response.text)

    if "MFA" in title:
        return {
            "status": "mfa_required",
            "message": "Garmin requested a six-digit verification code. Check your email and enter it below.",
            "challengeState": serialize_challenge_state(
                {
                    "session": session,
                    "signinParams": GARMIN_SIGNIN_PARAMS,
                    "lastResponseText": signin_response.text,
                    "lastResponseUrl": signin_response.url,
                }
            ),
            "accountEmail": email,
        }

    if title != "Success":
        page_error = extract_page_error(signin_response.text)
        raise GarminConnectAuthenticationError(
            page_error or "Garmin did not accept the email and password you entered."
        )

    token_store = complete_login(session, signin_response)
    return restore_session_from_tokens(email, token_store)


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
    configure_retry_session(client.garth.sess, retries=SYNC_RETRY_COUNT, backoff_factor=0.35)
    client.garth.timeout = SYNC_REQUEST_TIMEOUT_SECONDS
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
        ValueError,
    ) as error:
        sys.stderr.write(str(error))
        raise SystemExit(1)
    except Exception as error:
        sys.stderr.write(str(error) or "The Garmin adapter failed unexpectedly.")
        raise SystemExit(1)


if __name__ == "__main__":
    main()
