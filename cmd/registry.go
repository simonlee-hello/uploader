package cmd

import "uploader/route"

// Thin aliases so existing cmd code and tests keep compiling against local names.
type BackendInfo = route.BackendInfo

func findBackend(name string) *BackendInfo { return route.FindBackend(name) }

func backendsFitting(size int64) []string { return route.BackendsFitting(size) }

func formatBackendTable() string { return route.FormatBackendTable() }
