{{/*
Expand the name of the chart.
*/}}
{{- define "cilium-vision.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "cilium-vision.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "cilium-vision.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "cilium-vision.labels" -}}
helm.sh/chart: {{ include "cilium-vision.chart" . }}
{{ include "cilium-vision.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "cilium-vision.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cilium-vision.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
API selector labels
*/}}
{{- define "cilium-vision.api.selectorLabels" -}}
{{ include "cilium-vision.selectorLabels" . }}
app.kubernetes.io/component: api
{{- end }}

{{/*
UI selector labels
*/}}
{{- define "cilium-vision.ui.selectorLabels" -}}
{{ include "cilium-vision.selectorLabels" . }}
app.kubernetes.io/component: ui
{{- end }}

{{/*
Redis selector labels
*/}}
{{- define "cilium-vision.redis.selectorLabels" -}}
{{ include "cilium-vision.selectorLabels" . }}
app.kubernetes.io/component: redis
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "cilium-vision.serviceAccountName" -}}
{{- if .Values.api.serviceAccount.create }}
{{- default (printf "%s-api" (include "cilium-vision.fullname" .)) .Values.api.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.api.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
API image with tag
*/}}
{{- define "cilium-vision.api.image" -}}
{{- printf "%s:%s" .Values.api.image.repository (default .Chart.AppVersion .Values.api.image.tag) }}
{{- end }}

{{/*
UI image with tag
*/}}
{{- define "cilium-vision.ui.image" -}}
{{- printf "%s:%s" .Values.ui.image.repository (default .Chart.AppVersion .Values.ui.image.tag) }}
{{- end }}

{{/*
Redis URL - auto-configure if redis is enabled and no explicit URL set
*/}}
{{- define "cilium-vision.redisUrl" -}}
{{- if .Values.api.env.redisUrl }}
{{- .Values.api.env.redisUrl }}
{{- else if .Values.redis.enabled }}
{{- printf "redis://%s-redis:6379" (include "cilium-vision.fullname" .) }}
{{- else }}
{{- "redis://localhost:6379" }}
{{- end }}
{{- end }}
