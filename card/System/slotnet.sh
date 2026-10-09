#!/bin/sh
# Home network verbs, sourced by slotlink.sh. Not meant to be run on its own:
# it reads the interface, process and control-socket settings that slotlink.sh
# has already resolved, and borrows its wifi_up, wait_dev, supplicant_stop,
# link_wait and trace helpers.
#
# wlan0 and wlan1 are one radio. slotlink.sh owns the link cable and clears
# whichever station session holds wlan0 before it claims it, so these verbs
# and a link session are never live at the same time.

SD="${SLOT_ROOT:-/mnt/sdcard}"
NET_FILE="${AGS_WIFI_FILE:-$SD/Config/wifi.txt}"
NET_CONF="$RUN/slotnet.conf"
NET_WAIT_S="${AGS_NET_WAIT_S:-20}"
NET_RESCAN="${AGS_NET_RESCAN:-4}"
UDHCPC="${AGS_UDHCPC:-udhcpc}"

CR=$(printf '\r')

net_trim() {
	t="$1"
	while :; do
		case "$t" in
		' '* | '	'* | "$CR"*) t="${t#?}" ;;
		*' ' | *'	' | *"$CR") t="${t%?}" ;;
		*) break ;;
		esac
	done
	printf '%s' "$t"
}

# Read one key out of the card's settings file. The accepted shape is the one
# slot-store's ini module writes and parses: `key = value`, blank lines and
# whole-line #, ; or [ comments skipped, split at the first =. A # inside a
# value is part of the value, so a password may contain one. A repeated key
# keeps the last entry, as the Rust reader does.
net_value() {
	[ -f "$NET_FILE" ] || return 1
	value=
	while IFS= read -r line || [ -n "$line" ]; do
		line=$(net_trim "$line")
		case "$line" in
		'' | '#'* | ';'* | '['*) continue ;;
		*=*) ;;
		*) continue ;;
		esac
		[ "$(net_trim "${line%%=*}")" = "$1" ] || continue
		value=$(net_trim "${line#*=}")
	done < "$NET_FILE"
	printf '%s' "$value"
}

# printf, not echo: a psk is arbitrary text and some shells' echo eats
# backslashes.
net_conf() {
	{
		printf 'ctrl_interface=%s\n' "$CTRL_DIR"
		printf 'network={\n'
		printf '\tssid="%s"\n' "$1"
		if [ -n "$2" ]; then
			printf '\tkey_mgmt=WPA-PSK\n'
			printf '\tpsk="%s"\n' "$2"
		else
			printf '\tkey_mgmt=NONE\n'
		fi
		printf '}\n'
	} > "$NET_CONF"
	/bin/chmod 600 "$NET_CONF"
}

net_up() {
	trace "net: asked"
	link_clear
	net_release
	wifi_up
	[ -d "$NET/wlan0" ] || return 1
	ssid=$(net_value ssid)
	psk=$(net_value psk)
	if [ -z "$ssid" ]; then
		trace "net: no ssid in $NET_FILE"
		echo "slotlink: no ssid in $NET_FILE" >&2
		return 4
	fi
	command -v "$WPA_SUPPLICANT" >/dev/null 2>&1 || return 1

	/bin/mkdir -p "$CTRL_DIR"
	net_conf "$ssid" "$psk"
	$IP link set wlan0 up 2>/dev/null || true
	$WPA_SUPPLICANT -B -i wlan0 -c "$NET_CONF" -Dnl80211 >/dev/null 2>&1 || return 1
	link_wait "$CTRL_DIR" wlan0 "$NET_WAIT_S" "$NET_RESCAN" "" || return 3
	trace "net: associated"
	$UDHCPC -i wlan0 -n -q >/dev/null 2>&1 || return 5
	trace "net: addressed"
	return 0
}

# Hand wlan0 back. slotlink.sh calls this before a link verb claims the radio,
# so link_clear stays exactly as upstream wrote it.
net_release() {
	supplicant_stop "$CTRL_DIR" wlan0
	$IP addr flush dev wlan0 2>/dev/null || true
	/bin/rm -f "$NET_CONF"
	return 0
}

net_down() {
	trace "net: down"
	net_release
}

net_status() {
	if [ ! -f "$NET_CONF" ]; then
		echo off
		return 0
	fi
	state=$($WPA_CLI -p "$CTRL_DIR" -i wlan0 status 2>/dev/null | sed -n 's/^wpa_state=//p')
	case "$state" in
	COMPLETED) echo up ;;
	'') echo off ;;
	*) echo joining ;;
	esac
	return 0
}
