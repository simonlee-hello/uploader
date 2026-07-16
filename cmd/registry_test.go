package cmd

import "testing"

func TestBackendMaxBytes(t *testing.T) {
	tmpf := findBackend("tmpf")
	if tmpf == nil || tmpf.MaxBytes() != 100*1024*1024 {
		t.Fatalf("tmpf limit: %+v %d", tmpf, tmpf.MaxBytes())
	}
	gof := findBackend("gof")
	if gof == nil || gof.MaxBytes() != 0 {
		t.Fatalf("gof should be unlimited")
	}
}

func TestBackendsFitting(t *testing.T) {
	alts := backendsFitting(150 * 1024 * 1024) // 150MB
	for _, a := range alts {
		if a == "tmpf" || a == "cnet" {
			t.Fatalf("%s should not fit 150MB", a)
		}
	}
	found := false
	for _, a := range alts {
		if a == "lit" || a == "gof" || a == "temp" {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected larger backends, got %v", alts)
	}
}
