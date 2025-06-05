{{- define "mta.tag" }}
{{- .Values.images.mta.tag | default .Chart.AppVersion }}
{{- end}}

{{- define "management.tag" }}
{{- .Values.images.management.tag | default .Chart.AppVersion }}
{{- end}}