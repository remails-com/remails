import { Alert, Button, Center, Group, Image, Modal, Stack, Stepper, Text, TextInput } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails.ts";
import { useForm } from "@mantine/form";
import { notifications } from "@mantine/notifications";
import { IconInfoCircle, IconX } from "@tabler/icons-react";
import { ReactNode, useEffect, useState } from "react";
import { TotpCode } from "../../types.ts";
import { useDisclosure } from "@mantine/hooks";

interface FormValues {
  code: string;
  description: string;
}

interface TotpSetupProps {
  opened: boolean;
  close: () => void;
}

export default function TotpSetup({ opened, close }: TotpSetupProps) {
  const {
    state: { user },
    dispatch,
  } = useRemails();
  const [activeStep, setActiveStep] = useState(0);
  const [showInfo, { open: openInfo, close: closeInfo }] = useDisclosure(false);
  const [qrSrc, setQrSrc] = useState<ReactNode>(null);

  const form = useForm<FormValues>({
    initialValues: {
      code: "",
      description: "",
    },
    validate: {
      code: (value) => {
        if (value.match(/^[0-9]{6}$/) === null) {
          return "Code must have 6 digits";
        }
      },
    },
  });

  useEffect(() => {
    if (opened) {
      setQrSrc(`/api/api_user/${user?.id}/totp/enroll?cacheInvalidator=${Math.random().toString(36).slice(2, 7)}`);
      setActiveStep(0);
    }
  }, [opened]);

  if (!user) {
    return null;
  }

  const submit = async (values: FormValues) => {
    const res = await fetch(`/api/api_user/${user.id}/totp/enroll`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(values),
    });
    if (res.status === 400) {
      form.setFieldError("code", "Invalid code");
      openInfo();
      return;
    }
    if (res.status !== 200) {
      notifications.show({
        title: "Error",
        message: "2FA method could not be activated",
        color: "red",
        autoClose: 20000,
        icon: <IconX size={20} />,
      });
      console.error(res);
      return;
    }
    notifications.show({
      title: "2FA method activated",
      message: "",
      color: "green",
    });

    const totpCode: TotpCode = await res.json();

    close();
    dispatch({ type: "add_totp_code", totpCode });
    form.reset();
    setActiveStep(0);
  };

  return (
    <Modal
      opened={opened}
      onClose={() => {
        close();
        closeInfo();
        form.reset();
      }}
      title="2FA Setup"
      size="lg"
      closeOnClickOutside={false}
      closeOnEscape={false}
    >
      <Stepper active={activeStep} onStepClick={setActiveStep}>
        <Stepper.Step label="Scan QR code">
          <Stack>
            <Text>Please scan the following QR code with an authenticator App of your choice</Text>
            <Center>
              <Image src={qrSrc} w="auto" fit="contain" maw="50%" />
            </Center>
            <Group justify="space-between" mt="xl">
              <Button onClick={close} variant="outline">
                Cancel
              </Button>
              <Button onClick={() => setActiveStep(1)}>Next</Button>
            </Group>
          </Stack>
        </Stepper.Step>
        <Stepper.Step label="Verify">
          <Text>Then, type in the code your App generates to confirm that it works</Text>
          <form onSubmit={form.onSubmit(submit)}>
            <Stack>
              <TextInput
                label="Description"
                placeholder="e.g. Google Authenticator (optional)"
                value={form.values.description}
                onChange={(event) => form.setFieldValue("description", event.currentTarget.value)}
              />
              <TextInput
                label="Code"
                placeholder="123456"
                required
                value={form.values.code}
                error={form.errors.code}
                onChange={(event) => form.setFieldValue("code", event.currentTarget.value)}
              />
              {showInfo && (
                <Alert icon={<IconInfoCircle />}>
                  <Text>
                    If the problem persists, please check that the time on your device is correct and try another app.
                  </Text>
                </Alert>
              )}
              <Button type="submit" loading={form.submitting}>
                Activate
              </Button>
            </Stack>
          </form>
        </Stepper.Step>
      </Stepper>
    </Modal>
  );
}
