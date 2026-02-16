import { Modals, useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useEffect, useMemo, useRef } from "react";
import { Animated, Modal, Pressable, View } from "react-native";

import { GlobalText, IconClose } from "~/components/foundation";

import { AuthenticateSheet } from "./AuthenticateSheet";
import { ConfirmSendSheet } from "./ConfirmSendSheet";
import { ConfirmSwapSheet, type SheetRef } from "./ConfirmSwapSheet";

import type React from "react";

type SheetDefinition = {
  component: React.ComponentType<any>;
  options?: {
    title?: string;
    disableClosing?: boolean;
  };
};

const sheets: Record<string, SheetDefinition> = {
  [Modals.Authenticate]: {
    component: AuthenticateSheet,
    options: {
      title: m["common.signin"](),
    },
  },
  [Modals.ConfirmSwap]: {
    component: ConfirmSwapSheet,
    options: {
      title: m["applets.convert.title"](),
    },
  },
  [Modals.ConfirmSend]: {
    component: ConfirmSendSheet,
    options: {
      title: m["modals.confirmSend.title"](),
    },
  },
};

export const RootSheet: React.FC = () => {
  const { modal, hideModal } = useApp();
  const translateY = useRef(new Animated.Value(400)).current;
  const sheetRef = useRef<SheetRef | null>(null);

  const activeModal = modal.modal;
  const modalProps = modal.props;

  const definition = useMemo(() => {
    if (!activeModal) return undefined;
    return sheets[activeModal];
  }, [activeModal]);

  useEffect(() => {
    if (!activeModal || !definition) return;
    translateY.setValue(400);
    Animated.spring(translateY, {
      toValue: 0,
      useNativeDriver: true,
      tension: 70,
      friction: 13,
    }).start();
  }, [activeModal, definition, translateY]);

  if (!activeModal || !definition) return null;

  const closeSheet = (triggerOnClose: boolean) => {
    Animated.timing(translateY, {
      toValue: 400,
      duration: 180,
      useNativeDriver: true,
    }).start(() => {
      if (triggerOnClose) {
        sheetRef.current?.triggerOnClose?.();
      }
      hideModal();
    });
  };

  const Component = definition.component;

  return (
    <Modal transparent visible animationType="fade" onRequestClose={() => closeSheet(true)}>
      <View className="flex-1 justify-end">
        <Pressable className="absolute inset-0 bg-primitives-gray-light-900/50" onPress={() => !definition.options?.disableClosing && closeSheet(true)} />

        <Animated.View
          style={{ transform: [{ translateY }] }}
          className="bg-surface-primary-rice rounded-t-2xl px-4 pb-8 pt-3 min-h-24"
        >
          <View className="flex-row items-center justify-between mb-4">
            <View className="w-8" />
            <GlobalText className="diatype-lg-medium text-ink-tertiary-500">
              {definition.options?.title || ""}
            </GlobalText>
            <Pressable
              accessibilityRole="button"
              onPress={() => closeSheet(true)}
              className="w-8 h-8 items-center justify-center"
            >
              <IconClose className="text-ink-tertiary-500" />
            </Pressable>
          </View>

          <Component ref={sheetRef} {...modalProps} closeSheet={() => closeSheet(false)} />
        </Animated.View>
      </View>
    </Modal>
  );
};
