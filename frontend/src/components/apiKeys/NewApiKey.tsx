import { useState } from "react";
import { useOrganizations } from "../../hooks/useOrganizations.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { CreatedApiKeyWithPassword, KeyRole } from "../../types.ts";
import { useForm } from "@mantine/form";
import { Alert, Anchor, Button, Group, Modal, Select, Stack, Stepper, Text, Textarea, Title } from "@mantine/core";
import { IconInfoCircle } from "@tabler/icons-react";
import { CopyableCode } from "../CopyableCode.tsx";
import { errorNotification } from "../../notify.tsx";
import { ROLE_LABELS } from "../../util.ts";

const ALL_KEY_ROLES: KeyRole[] = ["read_only", "maintainer"];
export function isValidKeyRole(value: string): value is KeyRole {
  return ALL_KEY_ROLES.includes(value as KeyRole);
}

export const roleSelectData: { value: KeyRole; label: string }[] = ALL_KEY_ROLES.map((role) => ({
  value: role,
  label: ROLE_LABELS[role],
}));

interface FormValues {
  description: string;
  role: KeyRole;
}

interface NewApiKeyProps {
  opened: boolean;
  close: () => void;
}

export function NewApiKey({ opened, close }: NewApiKeyProps) {
  const [activeStep, setActiveStep] = useState(0);
  const { currentOrganization } = useOrganizations();
  const [newApiKey, setNewApiKey] = useState<CreatedApiKeyWithPassword | null>(null);
  const { dispatch } = useRemails();

  const form = useForm<FormValues>({
    validateInputOnBlur: true,
    initialValues: {
      description: "",
      role: "maintainer",
    },
    validate: {
      role: (value) => (isValidKeyRole(value) ? null : "Invalid role"),
    },
  });

  if (!currentOrganization) {
    return null;
  }

  const create = async (values: FormValues) => {
    const res = await fetch(`/api/organizations/${currentOrganization.id}/api_keys`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status !== 201) {
      errorNotification("API key could not be created");
      console.error(res);
      return;
    }
    const newApiKey = await res.json();
    setNewApiKey(newApiKey);
    dispatch({ type: "add_api_key", apiKey: { ...newApiKey, cleartext_password: undefined } });
    setActiveStep(1);
  };

  const credentials = `${newApiKey?.id}:${newApiKey?.password}`;
  const base64_credentials = `Authorization: Basic ${btoa(credentials)}`;

  return (
    <>
      <Modal
        opened={opened}
        onClose={activeStep === 0 ? close : () => {}}
        withCloseButton={activeStep === 0}
        title={
          <Title order={3} component="span">
            Create new API key
          </Title>
        }
        size="lg"
        padding="xl"
      >
        <Stepper active={activeStep} onStepClick={setActiveStep}>
          <Stepper.Step label="Create" allowStepSelect={false}>
            <form onSubmit={form.onSubmit(create)}>
              <Stack>
                <Textarea
                  data-autofocus
                  label="Description"
                  key={form.key("description")}
                  value={form.values.description}
                  error={form.errors.description}
                  onChange={(event) => form.setFieldValue("description", event.currentTarget.value)}
                />
                <Select
                  data-autofocus
                  label="Organization role"
                  placeholder="Pick a role"
                  data={roleSelectData}
                  value={form.values.role}
                  error={form.errors.role}
                  onChange={(value) => value && isValidKeyRole(value) && form.setFieldValue("role", value)}
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
          <Stepper.Step label="Configure" allowStepSelect={false}>
            <Stack>
              <Text>
                To authenticate using the API key, use{" "}
                <Anchor
                  href="https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Authentication#basic_authentication_scheme"
                  target="_blank"
                  inline
                >
                  HTTP Basic Auth
                </Anchor>{" "}
                with the API Key ID as username and the password below as password.
              </Text>
              <CopyableCode label="API Key ID">{newApiKey?.id ?? ""}</CopyableCode>
              <CopyableCode label="Password">{newApiKey?.password ?? ""}</CopyableCode>
              <Text>Include the following authorization header in your API requests to use these credentials:</Text>
              <CopyableCode>{base64_credentials}</CopyableCode>
              <Text fw="bold">
                Do not share this authorization code publicly because it contains the key ID and password shown above!
              </Text>
              <Alert
                variant="light"
                title="Save the password or authentication header somewhere safe!"
                icon={<IconInfoCircle />}
              >
                The password and authentication header will only be shown once. After you closed this window, we cannot
                show it again. If you lose it, you can simply create a new API key and delete the old one if necessary.
              </Alert>
              <Group justify="flex-end">
                <Button
                  onClick={() => {
                    setActiveStep(0);
                    setNewApiKey(null);
                    close();
                  }}
                >
                  Done
                </Button>
              </Group>
            </Stack>
          </Stepper.Step>
        </Stepper>
      </Modal>
    </>
  );
}
