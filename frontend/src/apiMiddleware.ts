import { Dispatch } from "react";
import { NavigationState } from "./hooks/useRouter";
import { Action } from "./types";
import { FullRouterState, Router } from "./router";

export default async function apiMiddleware(navState: NavigationState, router: Router, dispatch: Dispatch<Action>): Promise<FullRouterState> {
    const newOrgId = navState.to.params.org_id;
    const newProjId = navState.to.params.proj_id;
    const newStreamId = navState.to.params.stream_id;

    const orgChanged = newOrgId !== navState.from.params.org_id && newOrgId !== null;
    const projChanged = newProjId !== navState.from.params.proj_id && newProjId !== null;
    const streamChanged = newStreamId !== navState.from.params.stream_id && newStreamId !== null;
    
    if (navState.state.config === null) {
        const config = await fetch("/api/config").then((res) => res.json());
        dispatch({ type: "set_config", config });
    }

    if (orgChanged) {
        const organizations = await fetch("/api/organizations").then((r) => r.json())
        dispatch({ type: "set_organizations", organizations });

        // navigate to the first organization if none is selected
        if (!navState.to.params.org_id && organizations.length > 0) {
            navState.to = router.navigate("projects", { org_id: organizations[0].id });
        }

        const projects = await fetch(`/api/organizations/${newOrgId}/projects`).then((res) => res.json());
        dispatch({ type: "set_projects", projects });

        if (!newProjId) {
            const orgDomains = await fetch(`/api/organizations/${newOrgId}/domains`).then((res) => res.json());
            dispatch({ type: "set_domains", domains: orgDomains });
        }
    }

    if (orgChanged || projChanged) {
        const streams = await fetch(`/api/organizations/${newOrgId}/projects/${newProjId}/streams`).then((res) => res.json());
        dispatch({ type: "set_streams", streams });

        const domains = await fetch(`/api/organizations/${newOrgId}/projects/${newProjId}/domains`).then((res) => res.json());
        dispatch({ type: "set_domains", domains });
    }

    if (orgChanged || projChanged || streamChanged) {
        const messages = await fetch(`/api/organizations/${newOrgId}/projects/${newProjId}/streams/${newStreamId}/messages`).then((res) => res.json());
        dispatch({ type: "set_messages", messages });

        const smtpCredentials = await fetch(`/api/organizations/${newOrgId}/projects/${newProjId}/streams/${newStreamId}/smtp_credentials`).then((res) => res.json());
        dispatch({ type: "set_credentials", credentials: smtpCredentials });
    }

    if (newOrgId === null) {
        dispatch({ type: "set_organizations", organizations: null });
        dispatch({ type: "set_projects", projects: null });
        dispatch({ type: "set_streams", streams: null });
        dispatch({ type: "set_messages", messages: null });
        dispatch({ type: "set_domains", domains: null });
        dispatch({ type: "set_credentials", credentials: null });
    }

    return navState.to;
}
