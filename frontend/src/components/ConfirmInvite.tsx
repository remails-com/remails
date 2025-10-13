import { Button, Group, LoadingOverlay, Modal, Text, Tooltip } from "@mantine/core";
import { useRemails } from "../hooks/useRemails.ts";
import { RemailsLogo } from "./RemailsLogo.tsx";
import { useEffect, useState } from "react";
import { RemailsError } from "../error/error.ts";
import { Invite } from "../types.ts";
import { errorNotification } from "../notify.tsx";

export function ConfirmInvite() {
  const {
    navigate,
    dispatch,
    state: {
      routerState: { params },
    },
  } = useRemails();
  const {
    state: { user },
  } = useRemails();

  const [invite, setInvite] = useState<Invite | null>(null);

  useEffect(() => {
    fetch(`/api/invite/${params.new_org_id}/${params.invite_id}/${params.password}`).then((res) => {
      if (!res.ok) {
        const error = new RemailsError(
          `Could not get invite for organization ${params.new_org_id} (${res.status} ${res.statusText})`,
          res.status
        );
        dispatch({ type: "set_error", error });
        throw error;
      }
      res.json().then(setInvite);
    });
  }, [dispatch, params]);

  if (!user) {
    return null;
  }

  const accept = async () => {
    const res = await fetch(`/api/invite/${params.new_org_id}/${params.invite_id}/${params.password}`, {
      method: "POST",
    });

    if (res.status !== 201) {
      errorNotification("Could not accept invite");
      console.error(res);
      return;
    }

    const newOrg = await res.json();
    dispatch({ type: "add_organization", organization: newOrg });
    navigate("projects", { org_id: newOrg.id });
  };

  const already_joined = user.org_roles.some((o) => o.org_id == invite?.organization_id);
  const is_expired = invite && new Date(invite.expires_at) < new Date();

  return (
    <Modal
      opened={true}
      onClose={() => {}}
      withCloseButton={false}
      centered
      overlayProps={{ backgroundOpacity: 0.55, blur: 3 }}
    >
      <Text size="xl" fw="bold">
        Welcome to
      </Text>
      <Group justify="center">
        <RemailsLogo />
      </Group>

      <Group pos="relative" mih="3em" my="lg">
        <LoadingOverlay visible={!invite} />
        {invite && (
          <Text>
            You've been invited by {invite.created_by_name} to join the{" "}
            <Text span fw="bold">
              {invite.organization_name}
            </Text>{" "}
            organization.
          </Text>
        )}
      </Group>
      <Group justify="flex-end">
        <Button variant="default" disabled={!invite} onClick={() => navigate("default")}>
          Cancel
        </Button>
        <Tooltip
          disabled={!already_joined && !is_expired}
          label={already_joined ? "You are already in this organization." : "Invite is expired."}
        >
          <Button type="submit" disabled={!invite || already_joined} onClick={accept}>
            Join organization
          </Button>
        </Tooltip>
      </Group>
    </Modal>
  );
}
