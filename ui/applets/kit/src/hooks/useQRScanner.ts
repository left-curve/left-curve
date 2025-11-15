import {
  BrowserMultiFormatReader,
  type Result,
  type Exception,
  type DecodeContinuouslyCallback,
  type DecodeHintType,
} from "@zxing/library";
import { useCallback, useEffect, useRef, useState } from "react";

type UseQRCodeReaderParameters = {
  paused?: boolean;
  timeBetweenScansMillis?: number;
  onDecodeResult?: (result: Result) => void;
  onDecodeError?: (error: Exception) => void;
  deviceId?: string;
  hints?: Map<DecodeHintType, unknown>;
  constraints?: MediaStreamConstraints;
};

const DEFAULT_CONSTRAINTS: MediaStreamConstraints = {
  video: { facingMode: "environment" },
  audio: false,
};

export function useQRCodeReader(parameters: UseQRCodeReaderParameters) {
  const {
    timeBetweenScansMillis = 300,
    paused = false,
    onDecodeError,
    onDecodeResult,
    deviceId,
    hints,
    constraints,
  } = parameters;

  const [hasInitialized, setHasInitialized] = useState(false);

  const { current: reader } = useRef(new BrowserMultiFormatReader(hints, timeBetweenScansMillis));

  const decodeResultHandlerRef = useRef(onDecodeResult);
  const decodeErrorHandlerRef = useRef(onDecodeError);
  const ref = useRef<HTMLVideoElement>(null);

  const decodeCallback = useCallback<DecodeContinuouslyCallback>((result, error) => {
    if (result) decodeResultHandlerRef.current?.(result);
    if (error) decodeErrorHandlerRef.current?.(error);
  }, []);

  useEffect(() => {
    const videoElement = ref.current;
    if (!videoElement) return;

    if (paused) {
      reader.reset();
      return;
    }

    const startAsync = async () => {
      try {
        if (deviceId) {
          await reader.decodeFromVideoDevice(deviceId, videoElement, decodeCallback);
        } else {
          await reader.decodeFromConstraints(
            constraints ?? DEFAULT_CONSTRAINTS,
            videoElement,
            decodeCallback,
          );
        }
        setHasInitialized(true);
      } catch (e: unknown) {
        decodeErrorHandlerRef.current?.(e as Exception);
      }
    };

    startAsync();

    return () => {
      reader.reset();
    };
  }, [paused, deviceId, constraints, reader, decodeCallback]);
  return { ref, hasInitialized };
}
