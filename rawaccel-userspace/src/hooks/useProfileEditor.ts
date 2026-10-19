import { useCallback, useState } from 'react';
import { splitSettingsDocument, type SettingsDocument } from '../settingsDocument';
import { defaultProfile, type Profile, type DeviceConfig, type AccelArgs, SETTINGS_SCHEMA_VERSION } from '../types';
import { renameProfileState, deleteProfileState } from '../profileState';
import type { Notify } from './useNotifications';

// Own the settings document and the editor state that must follow its profiles.
export function useProfileEditor(setModalState: Notify) {
  const [profiles, setProfiles] = useState<Profile[]>([defaultProfile()]);
  const [activeProfileIndex, setActiveProfileIndex] = useState(0);
  const activeProfile = profiles[activeProfileIndex] || profiles[0];

  const [deviceConfig, setDeviceConfig] = useState<DeviceConfig[]>([]);
  const [settingsMetadata, setSettingsMetadata] = useState<Record<string, unknown>>({ schema_version: SETTINGS_SCHEMA_VERSION });

  const [activeCurveAxis, setActiveCurveAxis] = useState<"x" | "y">("x");
  const linkXY = activeProfile.speed_processor_args.whole;
  const replaceDocument = useCallback((document: SettingsDocument) => {
    const { profiles, devices, metadata } = splitSettingsDocument(document);
    setSettingsMetadata(metadata);
    if (profiles && profiles.length > 0) {
      setProfiles(profiles);
      setActiveProfileIndex(0);
      setActiveCurveAxis('x');
    }
    if (devices) setDeviceConfig(devices);
  }, []);
  // State Update Helpers
  const updateActiveProfile = (updater: (p: Profile) => Profile) => {
    setProfiles(prev => prev.map((p, i) => i === activeProfileIndex ? updater(p) : p));
  };

  const updateProfile = <K extends keyof Profile>(key: K, value: Profile[K]) => {
    updateActiveProfile(p => ({ ...p, [key]: value }));
  };

  const renameProfile = (newName: string) => {
    try {
      const next = renameProfileState(profiles, deviceConfig, activeProfileIndex, newName);
      setProfiles(next.profiles);
      setDeviceConfig(next.devices);
    } catch (error) {
      setModalState({ isOpen: true, title: "Profile Name Already Used", message: String(error) });
    }
  };

  const updateSpeed = (key: string, value: number) => {
    updateActiveProfile(p => ({ ...p, speed_processor_args: { ...p.speed_processor_args, [key]: value } }));
  };

  const updateAccel = <K extends keyof AccelArgs>(key: K, value: AccelArgs[K]) => {
    updateActiveProfile(p => {
      const next = { ...p };
      if (linkXY) {
        next.accel_x = { ...next.accel_x, [key]: value };
        next.accel_y = { ...next.accel_y, [key]: value };
      } else {
        if (activeCurveAxis === "x") next.accel_x = { ...next.accel_x, [key]: value };
        if (activeCurveAxis === "y") next.accel_y = { ...next.accel_y, [key]: value };
      }
      return next;
    });
  };

  const updateLookup = (updater: (args: AccelArgs) => AccelArgs) => {
    updateActiveProfile((profile) => {
      if (linkXY) {
        return {
          ...profile,
          accel_x: updater(profile.accel_x),
          accel_y: updater(profile.accel_y),
        };
      }

      return activeCurveAxis === "x"
        ? { ...profile, accel_x: updater(profile.accel_x) }
        : { ...profile, accel_y: updater(profile.accel_y) };
    });
  };

  const activeAccel = activeCurveAxis === "x" ? activeProfile.accel_x : activeProfile.accel_y;

  const handleLinkXY = () => {
    setActiveCurveAxis("x");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: true },
      accel_y: JSON.parse(JSON.stringify(p.accel_x))
    }));
  };

  const handleSelectXAxis = () => {
    setActiveCurveAxis("x");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: false }
    }));
  };

  const handleSelectYAxis = () => {
    setActiveCurveAxis("y");
    updateActiveProfile(p => ({
      ...p,
      speed_processor_args: { ...p.speed_processor_args, whole: false }
    }));
  };


  const addProfile = () => {
  if (profiles.length < 5) {
    const newProfile = defaultProfile();
    let nameIndex = profiles.length + 1;
    let candidateName = `Profile ${nameIndex}`;
    while (profiles.some(p => p.name.toLowerCase() === candidateName.toLowerCase())) {
      nameIndex++;
      candidateName = `Profile ${nameIndex}`;
    }
    newProfile.name = candidateName;
    const nextProfiles = [...profiles, newProfile];
    setProfiles(nextProfiles);
    setActiveProfileIndex(nextProfiles.length - 1);
    setActiveCurveAxis("x");
    return true;
  } else {
    setModalState({ isOpen: true, title: "Profile Limit Reached", message: "Maximum 5 profiles allowed." });
  }
    return false;
  };
  const selectProfile = (index: number) => {
    setActiveProfileIndex(index);
    setActiveCurveAxis('x');
  };
  const deleteProfile = (idx: number) => {
    const next = deleteProfileState(profiles, deviceConfig, activeProfileIndex, idx);
    setProfiles(next.profiles);
    setDeviceConfig(next.devices);
    setActiveProfileIndex(next.activeIndex);
    if (activeProfileIndex === idx) setActiveCurveAxis("x");
  };
  return { profiles, activeProfileIndex, replaceDocument,
    activeProfile, deviceConfig, setDeviceConfig, settingsMetadata,
    activeCurveAxis, linkXY, activeAccel, updateActiveProfile,
    updateProfile, renameProfile, updateSpeed, updateAccel, updateLookup,
    handleLinkXY, handleSelectXAxis, handleSelectYAxis, addProfile, selectProfile, deleteProfile };
}
export type ProfileEditor = ReturnType<typeof useProfileEditor>;
