import { Alert, Button, Group, Modal, Select, Stack, Stepper, Text, Title } from "@mantine/core";
import { CreatedInvite, Invite, Role } from "../../types";
import { CopyableCode } from "../CopyableCode";
import { formatDateTime, ROLE_LABELS } from "../../util";
import { useState } from "react";
import { useOrganizations } from "../../hooks/useOrganizations";
import { errorNotification } from "../../notify";
import { useSelector } from "../../hooks/useSelector";
import { useForm } from "@mantine/form";
import { IconInfoCircle } from "@tabler/icons-react";

interface NewInviteProps {
  opened: boolean;
  close: () => void;
  onNewInvite: (invite: Invite) => void;
}

const ALL_ROLES: Role[] = ["read_only", "maintainer", "admin"];
export function isValidRole(value: string): value is Role {
  return ALL_ROLES.includes(value as Role);
}

export const roleSelectData: { value: Role; label: string }[] = ALL_ROLES.map((role) => ({
  value: role,
  label: ROLE_LABELS[role],
}));

export const ROLE_INFO = (
  <Alert icon={<IconInfoCircle />} color="gray">
    <Text>
      <Text fw="bold" span>
        Read-only:{" "}
      </Text>
      can view the organization, including its projects, messages, domains, etc.
    </Text>
    <Text>
      <Text fw="bold" span>
        Maintainer:{" "}
      </Text>
      in addition to read-only access, can also create and edit projects, messages, and domains within the organization.
    </Text>
    <Text>
      <Text fw="bold" span>
        Admin:{" "}
      </Text>
      in addition to maintainer access, can also edit organization settings, including inviting and removing members.
    </Text>
  </Alert>
);

interface FormValues {
  role: Role;
}

export default function NewInvite({ opened, close, onNewInvite }: NewInviteProps) {
  const { currentOrganization } = useOrganizations();
  const [activeStep, setActiveStep] = useState(0);
  const user = useSelector((state) => state.user);
  const [invite, setInvite] = useState<CreatedInvite | null>(null);

  const form = useForm<FormValues>({
    validateInputOnBlur: true,
    initialValues: {
      role: "read_only",
    },
    validate: {
      role: (value) => (isValidRole(value) ? null : "Invalid role"),
    },
  });

  if (!currentOrganization) {
    return null;
  }

  const createInvite = async (values: FormValues) => {
    const res = await fetch(`/api/invite/${currentOrganization.id}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values.role),
    });

    if (res.status !== 201) {
      errorNotification("Could not create invite");
      console.error(res);
      return;
    }

    const invite = await res.json();
    invite.created_by_name = user.name;
    setInvite(invite);
    onNewInvite(invite);
    setActiveStep(1);
  };

  return (
    <Modal
      opened={opened}
      onClose={activeStep === 0 ? close : () => {}}
      withCloseButton={activeStep === 0}
      title={
        <Title order={3} component="span">
          Create new invite link
        </Title>
      }
      size="lg"
      padding="xl"
    >
      <Stepper active={activeStep} onStepClick={setActiveStep}>
        <Stepper.Step label="Create" allowStepSelect={false}>
          <form onSubmit={form.onSubmit(createInvite)}>
            <Stack>
              Please select which role the invite link should give:
              {ROLE_INFO}
              <Select
                data-autofocus
                label="Organization role"
                placeholder="Pick a role"
                data={roleSelectData}
                value={form.values.role}
                error={form.errors.role}
                onChange={(value) => value && isValidRole(value) && form.setFieldValue("role", value)}
                my="sm"
              />
              <Group justify="space-between">
                <Button onClick={close} variant="outline">
                  Cancel
                </Button>
                <Button type="submit" loading={form.submitting}>
                  Create
                </Button>
              </Group>
            </Stack>
          </form>
        </Stepper.Step>
        <Stepper.Step label="Share" allowStepSelect={false}>
          Share this link with the person you want to add to this organization:
          <CopyableCode props={{ my: "xs" }}>
            {`${window.location.protocol}//${window.location.host}/invite/${invite?.organization_id}/${invite?.id}/${invite?.password}`}
          </CopyableCode>
          <Text size="sm" c="dimmed">
            This link is valid until {invite ? formatDateTime(invite.expires_at) : "..."} and can only be used once.
          </Text>
          <Group mt="md" justify="flex-end">
            <Button
              onClick={() => {
                setActiveStep(0);
                setInvite(null);
                close();
              }}
            >
              Done
            </Button>
          </Group>
        </Stepper.Step>
      </Stepper>
    </Modal>
  );
}
