package main

import "fmt"

// jsonString quotes s as a JSON string using only fmt (no encoding/json,
// strconv or strings), so this file stays inside the lab's allow-list and
// also compiles under gc.
func jsonString(s string) string {
	out := "\""
	for _, r := range s {
		switch {
		case r == '"':
			out += "\\\""
		case r == '\\':
			out += "\\\\"
		case r == '\n':
			out += "\\n"
		case r == '\r':
			out += "\\r"
		case r == '\t':
			out += "\\t"
		case r < 0x20:
			out += fmt.Sprintf("\\u%04x", r)
		default:
			out += string(r)
		}
	}
	return out + "\""
}

// SetGreen schedules light i to turn green after delaySec seconds.
func SetGreen(i, delaySec int) {
	fmt.Printf("@city {\"op\":\"green\",\"i\":%d,\"delay\":%d}\n", i, delaySec)
}

// Lamp turns lamp i on or off.
func Lamp(i int, on bool) {
	fmt.Printf("@city {\"op\":\"lamp\",\"i\":%d,\"on\":%t}\n", i, on)
}

// Say shows msg in the city's speech bubble.
func Say(msg string) {
	fmt.Printf("@city {\"op\":\"say\",\"text\":%s}\n", jsonString(msg))
}
