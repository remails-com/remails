{{- /*
    Base64 Encode the essential "username:password" key

    Required keys:
      username
      password
*/ -}}
{{- define "regcred.b64Creds" -}}
  {{- printf "%s:%s" .username .password | b64enc }}
{{- end }}

{{- define "regcred.regCred" -}}
    {{- $secret := include "regcred.b64Creds" . }}
    {{- printf "\"%s\":{\"auth\": \"%s\"}" .url ($secret) }}
{{- end }}

{{- define "regcred.registryCredentials" -}}
    {{- $total := "" }}
    {{- range $rc := . }}
        {{- $rcTmp := include "regcred.regCred" . }}
        {{- $total = join "," (list $total $rcTmp) }}
        {{- $total = trimPrefix "," $total }}
    {{- end }}
    {{- printf "{\"auths\":{%s}}" $total | b64enc }}
{{- end }}
