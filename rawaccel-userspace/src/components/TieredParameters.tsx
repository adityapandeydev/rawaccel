import { NumberInput } from './NumberInput';
import { ProcessingControl } from './ProcessingControl';
import type { ProfileEditor } from '../hooks/useProfileEditor';

function TransitionField({ label, value, onChange, unit, stage, step = "0.1" }: {
  label: string; value: number; onChange: (value: number) => void; unit: string; stage: 1 | 2; step?: string;
}) {
  return <div className="input-group transition-field"><label className="input-label">{label}</label>
    <NumberInput className="input-field" aria-label={`Transition ${stage} ${label}`} value={value} step={step} onChange={event => onChange(parseFloat(event.target.value) || 0)} />
    <span className="field-unit">{unit}</span></div>;
}

export function TieredParameters({ editor }: { editor: ProfileEditor }) {
  const { activeAccel: args, updateAccel } = editor;
  const linear = (args.t_type || 'linear') === 'linear';
  return <div className="tiered-controls">
    <div className="input-group"><span className="input-label">Subtype</span><div className="segmented" aria-label="Tiered subtype">
      {(['linear', 'natural'] as const).map(type => <button key={type} className="segment" aria-pressed={(args.t_type || 'linear') === type} onClick={() => {
        updateAccel('t_type', type);
        if (type === 'linear') updateAccel('gain', false);
      }}>{type === 'linear' ? 'Linear' : 'Natural'}</button>)}
    </div></div>
    <ProcessingControl editor={editor} />
    <div className="stage-levels">
      {(['tiered_multiplier1', 'tiered_multiplier2', 'tiered_multiplier3'] as const).map((key, index) =>
        <label className="stage-level" key={key}><span>{['Base', 'Stage 1', 'Stage 2'][index]}</span>
          <div><NumberInput className="input-field" precision={2} aria-label={`Tier ${index + 1} Multiplier`} value={args[key]} step="0.1" onChange={event => updateAccel(key, parseFloat(event.target.value) || 0)} /><span>×</span></div>
        </label>)}
    </div>
    <section className="transition-section"><h3>Transition 1</h3>
      <TransitionField stage={1} label="Start" value={args.tiered_input_offset1} unit="counts/ms" onChange={value => {
        const maximum = args.tiered_input_offset2 - (linear ? args.tiered_transition1 : 0);
        updateAccel('tiered_input_offset1', Math.max(0, Math.min(value, maximum)));
      }} />
      {linear ? <TransitionField stage={1} label="Width" value={args.tiered_transition1} unit="counts/ms" onChange={value =>
        updateAccel('tiered_transition1', Math.max(0, Math.min(value, args.tiered_input_offset2 - args.tiered_input_offset1)))} />
        : <TransitionField stage={1} step="0.01" label="Decay rate" value={args.tiered_decay_rate1} unit="" onChange={value => updateAccel('tiered_decay_rate1', value)} />}
    </section>
    <section className="transition-section"><h3>Transition 2</h3>
      <TransitionField stage={2} label="Start" value={args.tiered_input_offset2} unit="counts/ms" onChange={value =>
        updateAccel('tiered_input_offset2', Math.max(value, args.tiered_input_offset1 + (linear ? args.tiered_transition1 : 0)))} />
      {linear ? <TransitionField stage={2} label="Width" value={args.tiered_transition2} unit="counts/ms" onChange={value => updateAccel('tiered_transition2', value)} />
        : <TransitionField stage={2} step="0.01" label="Decay rate" value={args.tiered_decay_rate2} unit="" onChange={value => updateAccel('tiered_decay_rate2', value)} />}
    </section>
  </div>;
}
