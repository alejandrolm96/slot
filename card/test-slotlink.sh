#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/System/slotlink.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
FAILS=0

fail() {
	echo "FAIL: $*"
	FAILS=$((FAILS + 1))
}

make_fake() {
	name="$1"
	out="$2"
	shift 2
	{
		echo '#!/bin/sh'
		echo "printf '%s\\n' \"\$*\" >> \"$out\""
		for line in "$@"; do echo "$line"; done
		echo 'exit 0'
	} > "$TMP/bin/$name"
	chmod 755 "$TMP/bin/$name"
}

reset() {
	rm -rf "$TMP/net" "$TMP/run" "$TMP/ctrl" "$TMP"/*.log "$TMP/bin" "$TMP/card"
	mkdir -p "$TMP/net/wlan0" "$TMP/net/wlan1" "$TMP/run" "$TMP/ctrl" "$TMP/modules" "$TMP/bin" \
		"$TMP/card/Config"
	: > "$TMP/modules/8821cs.ko"
	if [ "$1" = yes ]; then
		echo "8821cs 2863104 0 - Live 0x0" > "$TMP/proc-modules"
	else
		: > "$TMP/proc-modules"
	fi
	make_fake wpa_cli "$TMP/cli.log" 'echo "wpa_state=${FAKE_STATE:-COMPLETED}"'
	make_fake wpa_supplicant "$TMP/sup.log"
	make_fake ip "$TMP/ip.log" \
		'case "$*" in *"addr show"*) [ -z "${FAKE_ADDR:-}" ] || echo "    inet ${FAKE_ADDR}/24 scope global wlan0" ;; esac'
	make_fake insmod "$TMP/mod.log"
	make_fake legacy "$TMP/legacy.log"
	make_fake udhcpc "$TMP/dhcp.log"
}

run() {
	shift
	PATH="$TMP/bin:$PATH" \
		AGS_MODULES="$TMP/modules" AGS_NET_SYS="$TMP/net" AGS_RUN="$TMP/run" \
		AGS_CTRL_DIR="$TMP/ctrl" AGS_PROC_MODULES="$TMP/proc-modules" \
		AGS_WPA_SUPPLICANT="$TMP/bin/wpa_supplicant" AGS_WPA_CLI="$TMP/bin/wpa_cli" \
		AGS_IP="$TMP/bin/ip" AGS_INSMOD="$TMP/bin/insmod" \
		AGS_UDHCPC="$TMP/bin/udhcpc" AGS_WIFI_FILE="$TMP/card/Config/wifi.txt" \
		AGS_NET_LIB="${LIB:-$HERE/System/slotnet.sh}" \
		AGS_LINK_WAIT_S="${WAIT_S:-1}" AGS_LINK_PIN_WAIT_S="${WAIT_S:-1}" \
		AGS_NET_WAIT_S="${WAIT_S:-1}" \
		sh "$SCRIPT" "$@"
}

reset yes
run absent link host && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "host exited $rc"
grep -q 'ap_scan=2' "$TMP/run/slotlink-ap.conf" || fail "host AP scans before starting"
grep -q 'frequency=5745' "$TMP/run/slotlink-ap.conf" || fail "host AP not on 5745"
grep -q 'mode=2' "$TMP/run/slotlink-ap.conf" || fail "host config is not an AP"
grep -q 'addr add 10.42.0.1/24 dev wlan1' "$TMP/ip.log" || fail "host not addressed"
[ -f "$TMP/run/slotlink.session" ] || fail "host left no session mark"

reset yes
export FAKE_STATE=SCANNING
run absent link host && rc=0 || rc=$?
unset FAKE_STATE
[ "$rc" = 1 ] || fail "dead AP exited $rc, want 1"
grep -q 'addr add' "$TMP/ip.log" 2>/dev/null && fail "dead AP was addressed"

reset yes
run absent link join && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "join exited $rc"
grep -q 'scan_freq=5745' "$TMP/run/slotlink-sta.conf" || fail "join not pinned"
grep -q 'addr add 10.42.0.2/24 dev wlan0' "$TMP/ip.log" || fail "join not addressed"

reset yes
export FAKE_STATE=SCANNING
run absent link join && rc=0 || rc=$?
unset FAKE_STATE
[ "$rc" = 3 ] || fail "join with no host exited $rc, want 3"
[ "$(grep -c -- '-i wlan0' "$TMP/sup.log")" = 2 ] || fail "join did not make two attempts"

reset yes
run absent link join >/dev/null
run absent link down && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "down exited $rc"
for f in slotlink-ap.conf slotlink-sta.conf slotlink.session; do
	[ -e "$TMP/run/$f" ] && fail "down left $f"
done

reset yes
run absent link cool && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "cool exited $rc"
[ -e "$TMP/mod.log" ] && fail "cool touched the driver"

reset no
run absent link warm >/dev/null || true
[ -e "$TMP/mod.log" ] && fail "insmod on BaseOS"

reset yes
make_fake wpa_cli "$TMP/cli.log" 'sleep 0.3' 'echo "wpa_state=SCANNING"'
t0=$(date +%s)
run absent link join >/dev/null && rc=0 || rc=$?
t1=$(date +%s)
[ "$rc" = 3 ] || fail "slow join exited $rc, want 3"
[ $((t1 - t0)) -le 4 ] || fail "a 2 s join took $((t1 - t0)) s with a slow wpa_cli"

reset yes
run absent link sideways 2>/dev/null && rc=0 || rc=$?
[ "$rc" = 2 ] || fail "bad verb exited $rc, want 2"

creds() {
	printf 'ssid = %s\npsk = %s\n' "$1" "$2" > "$TMP/card/Config/wifi.txt"
}

reset yes
creds Home secret
run absent net up && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "net up exited $rc"
grep -Fq 'ssid="Home"' "$TMP/run/slotnet.conf" || fail "net up lost the ssid"
grep -Fq 'psk="secret"' "$TMP/run/slotnet.conf" || fail "net up lost the psk"
grep -Fq 'key_mgmt=WPA-PSK' "$TMP/run/slotnet.conf" || fail "net up is not WPA-PSK"
grep -q -- '-i wlan0' "$TMP/sup.log" || fail "net up started no supplicant on wlan0"
grep -q -- '-i wlan0' "$TMP/dhcp.log" || fail "net up asked for no lease"

# A psk is a value, never syntax. Nothing the shell or ini treats as special
# may be eaten on the way to the config, and wpa_supplicant takes the last
# quote on the line, so even a quote survives the round trip.
for psk in 'ab#cd*ef' 'back\slash' 'dollar$HOME' 'back`tick`' "single'quote" \
	'double"quote"in' 'spaces in it' 'equals=sign' 'semi;colon' 'bracket[s]'; do
	reset yes
	creds Home "$psk"
	run absent net up >/dev/null 2>&1 && rc=0 || rc=$?
	[ "$rc" = 0 ] || fail "net up with the psk [$psk] exited $rc"
	got=$(sed -n 's/^	psk="\(.*\)"$/\1/p' "$TMP/run/slotnet.conf")
	[ "$got" = "$psk" ] || fail "the psk [$psk] reached the config as [$got]"
done

# An ssid is a value too, and keeps the spaces inside it.
reset yes
creds 'My Home 5G' secret
run absent net up >/dev/null && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "net up with a spaced ssid exited $rc"
grep -Fq 'ssid="My Home 5G"' "$TMP/run/slotnet.conf" || fail "the ssid lost its spaces"

reset yes
printf '# home\n\nssid = Home\n; aside\npsk = secret\n' > "$TMP/card/Config/wifi.txt"
run absent net up >/dev/null && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "net up with comments exited $rc"
grep -Fq 'ssid="Home"' "$TMP/run/slotnet.conf" || fail "a comment line confused the reader"

reset yes
creds Open ''
run absent net up >/dev/null && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "net up on an open network exited $rc"
grep -Fq 'key_mgmt=NONE' "$TMP/run/slotnet.conf" || fail "an open network asked for a psk"

reset yes
run absent net up 2>/dev/null && rc=0 || rc=$?
[ "$rc" = 4 ] || fail "net up with no credentials exited $rc, want 4"
[ -e "$TMP/sup.log" ] && fail "net up started a supplicant with no credentials"
[ -e "$TMP/dhcp.log" ] && fail "net up asked for a lease with no credentials"

reset yes
creds Home secret
export FAKE_STATE=SCANNING
run absent net up >/dev/null 2>&1 && rc=0 || rc=$?
unset FAKE_STATE
[ "$rc" = 3 ] || fail "net up that never associates exited $rc, want 3"
[ -e "$TMP/dhcp.log" ] && fail "net up leased without associating"

reset yes
creds Home secret
run absent net up >/dev/null
run absent net down && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "net down exited $rc"
[ -e "$TMP/run/slotnet.conf" ] && fail "net down left the conf behind"
grep -q 'addr flush dev wlan0' "$TMP/ip.log" || fail "net down did not clear wlan0"

reset yes
run absent net status > "$TMP/status.log"
grep -Fqx off "$TMP/status.log" || fail "status with no session is not off"

# Associated is not the same as reachable. DHCP takes seconds after the join,
# and anything acting on "up" before the address lands fails on a name lookup.
reset yes
creds Home secret
run absent net up >/dev/null
run absent net status > "$TMP/status.log"
grep -Fqx joining "$TMP/status.log" || fail "an associated card with no address is not joining"

reset yes
creds Home secret
run absent net up >/dev/null
export FAKE_ADDR=192.168.1.50
run absent net status > "$TMP/status.log"
unset FAKE_ADDR
grep -Fqx up "$TMP/status.log" || fail "an addressed card is not up"

# One radio: every link verb takes wlan0 back from the home network. The
# release happens in the dispatcher, so link_clear stays as upstream wrote it.
for verb in host join down; do
	reset yes
	creds Home secret
	run absent net up >/dev/null
	[ -f "$TMP/run/slotnet.conf" ] || fail "net up left no session to release"
	: > "$TMP/ip.log"
	run absent link "$verb" >/dev/null 2>&1 || true
	[ -e "$TMP/run/slotnet.conf" ] && fail "link $verb left the Wi-Fi session running"
	grep -q 'addr flush dev wlan0' "$TMP/ip.log" || fail "link $verb never released wlan0"
done

reset yes
run absent net sideways 2>/dev/null && rc=0 || rc=$?
[ "$rc" = 2 ] || fail "bad net verb exited $rc, want 2"

# The net verbs live in a second file. Losing it costs those verbs and
# nothing else: the link cable is still a link cable.
reset yes
creds Home secret
export LIB="$TMP/absent-slotnet.sh"
run absent net up 2>"$TMP/err.log" && rc=0 || rc=$?
[ "$rc" = 1 ] || fail "net up without slotnet.sh exited $rc, want 1"
grep -q 'is missing' "$TMP/err.log" || fail "net up without slotnet.sh said nothing useful"
[ -e "$TMP/sup.log" ] && fail "net up without slotnet.sh still started a supplicant"
run absent link host >/dev/null && rc=0 || rc=$?
unset LIB
[ "$rc" = 0 ] || fail "link host without slotnet.sh exited $rc"

if [ "$FAILS" -eq 0 ]; then
	echo "slotlink: all passed"
else
	echo "slotlink: $FAILS failed"
	exit 1
fi
