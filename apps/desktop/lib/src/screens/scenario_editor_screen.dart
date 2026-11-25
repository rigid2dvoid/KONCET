import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/providers.dart';

class ScenarioEditorScreen extends ConsumerStatefulWidget {
  const ScenarioEditorScreen({
    super.key,
    required this.projectId,
    required this.scenarioId,
  });

  final String projectId;
  final String scenarioId;

  @override
  ConsumerState<ScenarioEditorScreen> createState() =>
      _ScenarioEditorScreenState();
}

class _ScenarioEditorScreenState extends ConsumerState<ScenarioEditorScreen> {
  Timer? _debounce;
  Map<String, dynamic>? _scenario;
  String _canonical = '';
  int _revision = 0;
  bool _initialised = false;
  bool _saving = false;
  bool _dirty = false;
  List<ValidationIssueDto> _issues = const [];
  String _saveLabel = 'Saved';

  @override
  void dispose() {
    _debounce?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scenario = ref.watch(scenarioProvider(widget.scenarioId));
    final project = ref.watch(projectProvider(widget.projectId));
    if (!_initialised && scenario.hasValue) {
      _initialise(scenario.value!);
    }
    if (!_initialised) {
      return scenario.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, stack) => KoncetErrorState(
          title: 'Scenario could not be loaded',
          message: error.toString(),
          onRetry: () => ref.invalidate(scenarioProvider(widget.scenarioId)),
        ),
        data: (_) => const SizedBox.shrink(),
      );
    }

    final theme = Theme.of(context);
    final projectName = project.value?.name ?? 'Project';
    final name = _scenario!['name'] as String? ?? 'Scenario';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KoncetPageHeader(
          breadcrumb: 'Projects / $projectName / Scenario',
          title: name,
          description:
              'Revision $_revision · schema v${_scenario!['schema_version'] ?? 1}. The Rust validator is authoritative; edits autosave after validation.',
          actions: [
            _SaveIndicator(saving: _saving, label: _saveLabel, dirty: _dirty),
            KoncetButton(
              label: 'Save now',
              icon: Icons.save_outlined,
              busy: _saving,
              onPressed: _dirty ? _save : null,
            ),
            IconButton(
              tooltip: 'Back to project',
              onPressed: () => context.go('/projects/${widget.projectId}'),
              icon: const Icon(Icons.close),
            ),
          ],
        ),
        const SizedBox(height: 16),
        if (_issues.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: KoncetReasonCallout(
              title: 'Scenario needs attention',
              message: _issues
                  .map(
                    (issue) => issue.path.isEmpty
                        ? issue.message
                        : '${issue.path}: ${issue.message}',
                  )
                  .take(4)
                  .join('\n'),
              tone: _issues.any((issue) => issue.severity == 'error')
                  ? KoncetStatusTone.danger
                  : KoncetStatusTone.warning,
            ),
          ),
        Expanded(
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _section(
                  'Identity',
                  Column(
                    children: [
                      _textField(
                        'Name',
                        _scenario!['name'],
                        (value) => _set('name', value),
                      ),
                      const SizedBox(height: 10),
                      _textField(
                        'Description',
                        _scenario!['description'] ?? '',
                        (value) => _set('description', value),
                        lines: 2,
                      ),
                    ],
                  ),
                ),
                _section('Seed policy', _seedPolicy()),
                _section('Build', _buildSection(theme)),
                _section('Service', _serviceSection(theme)),
                _section('Proxy', _proxySection()),
                _section('Firewall', _firewallSection(theme)),
                _section('Fault injection', _faultSection(theme)),
                _section('Limits and monitoring', _limitsSection()),
                if (_canonical.isNotEmpty)
                  KoncetSectionCard(
                    title: 'Canonical JSON',
                    description: 'The stored form is sorted and stable. Editing this preview is not supported.',
                    child: ExpansionTile(
                      tilePadding: EdgeInsets.zero,
                      title: const Text('Show canonical document'),
                      children: [
                        Container(
                          width: double.infinity,
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: SelectableText(
                            _canonical,
                            style: context.mono.small,
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _section(String title, Widget child) => Padding(
    padding: const EdgeInsets.only(bottom: 16),
    child: KoncetSectionCard(title: title, child: child),
  );

  Widget _seedPolicy() {
    final seed = _scenario!['seed_policy'] as Map<String, dynamic>;
    final mode = seed['mode'] as String? ?? 'random';
    return Wrap(
      spacing: 16,
      runSpacing: 12,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        SizedBox(
          width: 200,
          child: DropdownButtonFormField<String>(
            initialValue: mode,
            decoration: const InputDecoration(labelText: 'Mode'),
            items: const [
              DropdownMenuItem(value: 'random', child: Text('Random')),
              DropdownMenuItem(value: 'fixed', child: Text('Fixed')),
            ],
            onChanged: (value) {
              if (value == null) return;
              setState(() {
                seed['mode'] = value;
                if (value == 'random') {
                  seed.remove('value');
                } else {
                  seed['value'] = seed['value'] ?? '42';
                }
              });
              _touch();
            },
          ),
        ),
        if (mode == 'fixed')
          SizedBox(
            width: 260,
            child: _textField(
              'Fixed seed',
              seed['value'] ?? '',
              (value) => _setIn(seed, 'value', value),
            ),
          ),
        const Text(
          'A run-level seed override takes precedence over this policy.',
        ),
      ],
    );
  }

  Widget _buildSection(ThemeData theme) {
    final build = _scenario!['build'] as Map<String, dynamic>;
    final profile = build['profile'] as String? ?? 'static-web';
    final injections = build['failure_injections'] as List<dynamic>? ?? [];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: 16,
          runSpacing: 12,
          children: [
            SizedBox(
              width: 220,
              child: DropdownButtonFormField<String>(
                initialValue: profile,
                decoration: const InputDecoration(labelText: 'Profile'),
                items: const [
                  DropdownMenuItem(
                    value: 'static-web',
                    child: Text('static-web'),
                  ),
                  DropdownMenuItem(
                    value: 'http-service',
                    child: Text('http-service'),
                  ),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setState(() => build['profile'] = value);
                  _touch();
                },
              ),
            ),
            SizedBox(
              width: 180,
              child: _textField(
                'Duration scale',
                '${build['duration_scale'] ?? 1.0}',
                (value) => _setDoubleIn(build, 'duration_scale', value),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Text('Failure injections', style: theme.textTheme.titleSmall),
            const Spacer(),
            TextButton.icon(
              onPressed: injections.length >= 2
                  ? null
                  : () {
                      setState(() {
                        build['failure_injections'] = [
                          ...injections,
                          {
                            'stage': 'build',
                            'at_percent': 40,
                            'message': 'Injected compiler failure',
                          },
                        ];
                      });
                      _touch();
                    },
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add injection'),
            ),
          ],
        ),
        for (var index = 0; index < injections.length; index++)
          _InjectionRow(
            injection: injections[index] as Map<String, dynamic>,
            onChanged: _touch,
            onRemove: () {
              setState(
                () => build['failure_injections'] = [
                  ...injections..removeAt(index),
                ],
              );
              _touch();
            },
          ),
      ],
    );
  }

  Widget _serviceSection(ThemeData theme) {
    final service = _scenario!['service'] as Map<String, dynamic>;
    final restart = service['restart_policy'] as Map<String, dynamic>;
    final health = service['health_check'] as Map<String, dynamic>;
    final env = service['env'] as Map<String, dynamic>? ?? {};
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: 16,
          runSpacing: 12,
          children: [
            _sizedField(
              'Name',
              service['name'],
              220,
              (v) => _setIn(service, 'name', v),
            ),
            _sizedField(
              'Command',
              service['command'],
              360,
              (v) => _setIn(service, 'command', v),
            ),
            _numberField(
              'Port',
              service['port'],
              140,
              (v) => _setIntIn(service, 'port', v),
            ),
            _numberField(
              'CPU millicores',
              service['cpu_millicores'],
              160,
              (v) => _setIntIn(service, 'cpu_millicores', v),
            ),
            _numberField(
              'Memory MB',
              service['memory_mb'],
              140,
              (v) => _setIntIn(service, 'memory_mb', v),
            ),
            _numberField(
              'Startup delay (s)',
              service['startup_delay_s'] ?? 0,
              170,
              (v) => _setIntIn(service, 'startup_delay_s', v),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Text('Restart policy', style: theme.textTheme.titleSmall),
        const SizedBox(height: 8),
        Wrap(
          spacing: 16,
          runSpacing: 12,
          children: [
            SizedBox(
              width: 200,
              child: DropdownButtonFormField<String>(
                initialValue: restart['policy'] as String? ?? 'on-failure',
                decoration: const InputDecoration(labelText: 'Policy'),
                items: const [
                  DropdownMenuItem(value: 'no', child: Text('No')),
                  DropdownMenuItem(
                    value: 'on-failure',
                    child: Text('On failure'),
                  ),
                  DropdownMenuItem(value: 'always', child: Text('Always')),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setState(() => restart['policy'] = value);
                  _touch();
                },
              ),
            ),
            SizedBox(
              width: 180,
              child: _numberField(
                'Max retries',
                restart['max_retries'],
                160,
                (v) => _setIntIn(restart, 'max_retries', v),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Text('Health check', style: theme.textTheme.titleSmall),
        const SizedBox(height: 8),
        Wrap(
          spacing: 16,
          runSpacing: 12,
          children: [
            _sizedField(
              'Path',
              health['path'],
              220,
              (v) => _setIn(health, 'path', v),
            ),
            _numberField(
              'Interval (s)',
              health['interval_s'],
              150,
              (v) => _setIntIn(health, 'interval_s', v),
            ),
            _numberField(
              'Timeout (s)',
              health['timeout_s'],
              150,
              (v) => _setIntIn(health, 'timeout_s', v),
            ),
            _numberField(
              'Retries',
              health['retries'],
              140,
              (v) => _setIntIn(health, 'retries', v),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Row(
          children: [
            Text('Environment', style: theme.textTheme.titleSmall),
            const Spacer(),
            TextButton.icon(
              onPressed: () => _addEnv(env),
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add variable'),
            ),
          ],
        ),
        for (final entry in env.entries.toList())
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Row(
              children: [
                Expanded(
                  child: TextFormField(
                    initialValue: entry.key,
                    readOnly: true,
                    decoration: const InputDecoration(labelText: 'Key'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextFormField(
                    initialValue: '${entry.value}',
                    decoration: const InputDecoration(labelText: 'Value'),
                    onChanged: (value) {
                      env[entry.key] = value;
                      _touch();
                    },
                  ),
                ),
                IconButton(
                  tooltip: 'Remove variable',
                  onPressed: () {
                    setState(() => env.remove(entry.key));
                    _touch();
                  },
                  icon: const Icon(Icons.remove_circle_outline, size: 18),
                ),
              ],
            ),
          ),
      ],
    );
  }

  Widget _proxySection() {
    final proxy = _scenario!['proxy'] as Map<String, dynamic>;
    final enabled = proxy['enabled'] as bool? ?? false;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text('Enable simulated proxy'),
          subtitle: const Text(
            'Represents a virtual listener that routes to the service port. No socket is opened.',
          ),
          value: enabled,
          onChanged: (value) {
            setState(() => proxy['enabled'] = value);
            _touch();
          },
        ),
        const SizedBox(height: 8),
        Opacity(
          opacity: enabled ? 1 : 0.55,
          child: Wrap(
            spacing: 16,
            runSpacing: 12,
            children: [
              _numberField(
                'Listener port',
                proxy['listener_port'],
                160,
                (v) => _setIntIn(proxy, 'listener_port', v),
              ),
              _sizedField(
                'Host',
                proxy['host'],
                240,
                (v) => _setIn(proxy, 'host', v),
              ),
              _sizedField(
                'Path',
                proxy['path'],
                160,
                (v) => _setIn(proxy, 'path', v),
              ),
              _numberField(
                'Target port',
                proxy['target_port'],
                150,
                (v) => _setIntIn(proxy, 'target_port', v),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _firewallSection(ThemeData theme) {
    final firewall = _scenario!['firewall'] as Map<String, dynamic>;
    final policy = firewall['default_policy'] as String? ?? 'deny';
    final rules = firewall['rules'] as List<dynamic>? ?? [];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          spacing: 16,
          runSpacing: 12,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            SizedBox(
              width: 220,
              child: DropdownButtonFormField<String>(
                initialValue: policy,
                decoration: const InputDecoration(labelText: 'Default policy'),
                items: const [
                  DropdownMenuItem(value: 'deny', child: Text('Deny')),
                  DropdownMenuItem(value: 'allow', child: Text('Allow')),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setState(() => firewall['default_policy'] = value);
                  _touch();
                },
              ),
            ),
            Text(
              'Only inbound TCP is evaluated in v0.1.',
              style: theme.textTheme.bodySmall,
            ),
            TextButton.icon(
              onPressed: rules.length >= 10
                  ? null
                  : () {
                      setState(() {
                        firewall['rules'] = [
                          ...rules,
                          {
                            'id': 'allow-proxy-${rules.length + 1}',
                            'action': 'allow',
                            'direction': 'inbound',
                            'protocol': 'tcp',
                            'port': 3000,
                            'source': 'proxy',
                          },
                        ];
                      });
                      _touch();
                    },
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add rule'),
            ),
          ],
        ),
        for (var index = 0; index < rules.length; index++)
          _FirewallRuleRow(
            rule: rules[index] as Map<String, dynamic>,
            onChanged: _touch,
            onRemove: () {
              setState(() => firewall['rules'] = [...rules..removeAt(index)]);
              _touch();
            },
          ),
      ],
    );
  }

  Widget _faultSection(ThemeData theme) {
    final faults = _scenario!['faults'] as List<dynamic>? ?? [];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(
              'At most three deterministic faults may be scheduled.',
              style: theme.textTheme.bodySmall,
            ),
            const Spacer(),
            PopupMenuButton<String>(
              tooltip: 'Add fault',
              enabled: faults.length < 3,
              onSelected: (kind) {
                setState(() {
                  _scenario!['faults'] = [...faults, _newFault(kind)];
                });
                _touch();
              },
              itemBuilder: (context) => const [
                PopupMenuItem(
                  value: 'process_crash',
                  child: Text('Process crash'),
                ),
                PopupMenuItem(
                  value: 'healthcheck_failure',
                  child: Text('Health check failure'),
                ),
                PopupMenuItem(
                  value: 'startup_stall',
                  child: Text('Startup stall'),
                ),
              ],
              child: const Padding(
                padding: EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.add, size: 16),
                    SizedBox(width: 4),
                    Text('Add fault'),
                  ],
                ),
              ),
            ),
          ],
        ),
        for (var index = 0; index < faults.length; index++)
          _FaultRow(
            fault: faults[index] as Map<String, dynamic>,
            onChanged: _touch,
            onRemove: () {
              setState(
                () => _scenario!['faults'] = [...faults..removeAt(index)],
              );
              _touch();
            },
          ),
      ],
    );
  }

  Widget _limitsSection() {
    final limits = _scenario!['limits'] as Map<String, dynamic>;
    final monitoring = _scenario!['monitoring'] as Map<String, dynamic>;
    return Wrap(
      spacing: 16,
      runSpacing: 12,
      children: [
        _numberField(
          'Max simulated seconds',
          limits['max_sim_seconds'],
          210,
          (v) => _setIntIn(limits, 'max_sim_seconds', v),
        ),
        SizedBox(
          width: 240,
          child: SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Stop on terminal failure'),
            value: limits['stop_on_terminal_failure'] as bool? ?? true,
            onChanged: (value) {
              setState(() => limits['stop_on_terminal_failure'] = value);
              _touch();
            },
          ),
        ),
        _numberField(
          'Sample interval (s)',
          monitoring['sample_interval_s'],
          190,
          (v) => _setIntIn(monitoring, 'sample_interval_s', v),
        ),
      ],
    );
  }

  Widget _sizedField(
    String label,
    Object? value,
    double width,
    ValueChanged<String> onChanged,
  ) => SizedBox(width: width, child: _textField(label, value, onChanged));

  Widget _numberField(
    String label,
    Object? value,
    double width,
    ValueChanged<String> onChanged,
  ) => SizedBox(
    width: width,
    child: TextFormField(
      initialValue: value?.toString() ?? '',
      keyboardType: TextInputType.number,
      decoration: InputDecoration(labelText: label),
      onChanged: onChanged,
    ),
  );

  Widget _textField(
    String label,
    Object? value,
    ValueChanged<String> onChanged, {
    int lines = 1,
  }) => TextFormField(
    initialValue: '${value ?? ''}',
    maxLines: lines,
    decoration: InputDecoration(labelText: label),
    onChanged: onChanged,
  );

  void _initialise(ScenarioDetailDto detail) {
    _scenario = jsonDecode(detail.scenarioJson) as Map<String, dynamic>;
    _canonical = detail.canonicalJson;
    _revision = detail.revision;
    _initialised = true;
  }

  void _set(String key, Object? value) {
    _scenario![key] = value;
    _touch();
  }

  void _setIn(Map<String, dynamic> map, String key, Object? value) {
    map[key] = value;
    _touch();
  }

  void _setIntIn(Map<String, dynamic> map, String key, String text) {
    final value = int.tryParse(text.trim());
    if (value == null) return;
    map[key] = value;
    _touch();
  }

  void _setDoubleIn(Map<String, dynamic> map, String key, String text) {
    final value = double.tryParse(text.trim());
    if (value == null) return;
    map[key] = value;
    _touch();
  }

  Future<void> _addEnv(Map<String, dynamic> env) async {
    final key = TextEditingController();
    final value = TextEditingController();
    final result = await showDialog<({String key, String value})>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Add environment variable'),
        content: SizedBox(
          width: 420,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: key,
                autofocus: true,
                decoration: const InputDecoration(
                  labelText: 'Key',
                  hintText: 'APP_ENV',
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: value,
                decoration: const InputDecoration(
                  labelText: 'Value',
                  hintText: 'simulation',
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              final name = key.text.trim();
              if (name.isEmpty) return;
              Navigator.of(context).pop((key: name, value: value.text));
            },
            child: const Text('Add'),
          ),
        ],
      ),
    );
    key.dispose();
    value.dispose();
    if (result == null) return;
    setState(() => env[result.key] = result.value);
    _touch();
  }

  void _touch() {
    _debounce?.cancel();
    setState(() {
      _dirty = true;
      _saveLabel = 'Unsaved';
    });
    _debounce = Timer(const Duration(milliseconds: 900), () {
      if (_dirty) unawaited(_save());
    });
  }

  Future<void> _save() async {
    if (_saving) return;
    setState(() {
      _saving = true;
      _saveLabel = 'Saving…';
    });
    try {
      final record = await ref
          .read(apiProvider)
          .updateScenario(widget.scenarioId, _revision, jsonEncode(_scenario));
      if (!mounted) return;
      setState(() {
        _revision = record.revision;
        _scenario = jsonDecode(record.scenarioJson) as Map<String, dynamic>;
        _canonical = record.canonicalJson;
        _saving = false;
        _dirty = false;
        _issues = const [];
        _saveLabel = 'Saved';
      });
      ref.invalidate(scenarioProvider(widget.scenarioId));
      ref.invalidate(projectScenariosProvider(widget.projectId));
      if (_dirty) _touch();
    } on BridgeError catch (error) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        _dirty = true;
        _saveLabel = 'Validation failed';
        _issues = error.issues;
      });
      if (error.issues.isEmpty) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(error.message)));
      }
    }
  }

  Map<String, dynamic> _newFault(String kind) => switch (kind) {
    'healthcheck_failure' => {'kind': kind, 'after_s': 5, 'duration_s': 10},
    'startup_stall' => {'kind': kind, 'at_s': 0, 'duration_s': 5},
    _ => {'kind': 'process_crash', 'after_s': 5, 'once': true, 'exit_code': 1},
  };
}

class _InjectionRow extends StatelessWidget {
  const _InjectionRow({
    required this.injection,
    required this.onChanged,
    required this.onRemove,
  });

  final Map<String, dynamic> injection;
  final VoidCallback onChanged;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          SizedBox(
            width: 140,
            child: DropdownButtonFormField<String>(
              initialValue: injection['stage'] as String? ?? 'build',
              decoration: const InputDecoration(labelText: 'Stage'),
              items: const [
                DropdownMenuItem(value: 'validate', child: Text('Validate')),
                DropdownMenuItem(value: 'prepare', child: Text('Prepare')),
                DropdownMenuItem(value: 'build', child: Text('Build')),
                DropdownMenuItem(value: 'package', child: Text('Package')),
              ],
              onChanged: (value) {
                if (value == null) return;
                injection['stage'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 120,
            child: TextFormField(
              initialValue: '${injection['at_percent'] ?? 40}',
              decoration: const InputDecoration(labelText: 'At %'),
              keyboardType: TextInputType.number,
              onChanged: (value) {
                final parsed = int.tryParse(value);
                if (parsed != null) {
                  injection['at_percent'] = parsed;
                  onChanged();
                }
              },
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: TextFormField(
              initialValue: '${injection['message'] ?? ''}',
              decoration: const InputDecoration(labelText: 'Message'),
              onChanged: (value) {
                injection['message'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          IconButton(
            tooltip: 'Remove injection',
            onPressed: onRemove,
            icon: const Icon(Icons.remove_circle_outline, size: 18),
          ),
        ],
      ),
    );
  }
}

class _FirewallRuleRow extends StatelessWidget {
  const _FirewallRuleRow({
    required this.rule,
    required this.onChanged,
    required this.onRemove,
  });

  final Map<String, dynamic> rule;
  final VoidCallback onChanged;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          SizedBox(
            width: 150,
            child: TextFormField(
              initialValue: '${rule['id'] ?? ''}',
              decoration: const InputDecoration(labelText: 'Rule ID'),
              onChanged: (value) {
                rule['id'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 120,
            child: DropdownButtonFormField<String>(
              initialValue: rule['action'] as String? ?? 'allow',
              decoration: const InputDecoration(labelText: 'Action'),
              items: const [
                DropdownMenuItem(value: 'allow', child: Text('Allow')),
                DropdownMenuItem(value: 'deny', child: Text('Deny')),
              ],
              onChanged: (value) {
                if (value == null) return;
                rule['action'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 130,
            child: DropdownButtonFormField<String>(
              initialValue: rule['protocol'] as String? ?? 'tcp',
              decoration: const InputDecoration(labelText: 'Protocol'),
              items: const [
                DropdownMenuItem(value: 'tcp', child: Text('TCP')),
                DropdownMenuItem(value: 'udp', child: Text('UDP')),
              ],
              onChanged: (value) {
                if (value == null) return;
                rule['protocol'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 110,
            child: TextFormField(
              initialValue: '${rule['port'] ?? ''}',
              decoration: const InputDecoration(labelText: 'Port'),
              keyboardType: TextInputType.number,
              onChanged: (value) {
                final parsed = int.tryParse(value);
                if (parsed != null) {
                  rule['port'] = parsed;
                  onChanged();
                }
              },
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 140,
            child: DropdownButtonFormField<String>(
              initialValue: rule['source'] as String? ?? 'proxy',
              decoration: const InputDecoration(labelText: 'Source'),
              items: const [
                DropdownMenuItem(value: 'proxy', child: Text('Proxy')),
                DropdownMenuItem(value: 'monitor', child: Text('Monitor')),
                DropdownMenuItem(value: 'external', child: Text('External')),
              ],
              onChanged: (value) {
                if (value == null) return;
                rule['source'] = value;
                onChanged();
              },
            ),
          ),
          const Spacer(),
          IconButton(
            tooltip: 'Remove rule',
            onPressed: onRemove,
            icon: const Icon(Icons.remove_circle_outline, size: 18),
          ),
        ],
      ),
    );
  }
}

class _FaultRow extends StatelessWidget {
  const _FaultRow({
    required this.fault,
    required this.onChanged,
    required this.onRemove,
  });

  final Map<String, dynamic> fault;
  final VoidCallback onChanged;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final kind = fault['kind'] as String? ?? 'process_crash';
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        children: [
          SizedBox(
            width: 180,
            child: DropdownButtonFormField<String>(
              initialValue: kind,
              decoration: const InputDecoration(labelText: 'Kind'),
              items: const [
                DropdownMenuItem(
                  value: 'process_crash',
                  child: Text('Process crash'),
                ),
                DropdownMenuItem(
                  value: 'healthcheck_failure',
                  child: Text('Health check'),
                ),
                DropdownMenuItem(
                  value: 'startup_stall',
                  child: Text('Startup stall'),
                ),
              ],
              onChanged: (value) {
                if (value == null) return;
                fault['kind'] = value;
                onChanged();
              },
            ),
          ),
          const SizedBox(width: 8),
          if (kind == 'startup_stall') ...[
            _number('at_s', 'At (s)'),
            _number('duration_s', 'Duration (s)'),
          ] else if (kind == 'healthcheck_failure') ...[
            _number('after_s', 'After (s)'),
            _number('duration_s', 'Duration (s)'),
          ] else ...[
            _number('after_s', 'After (s)'),
            SizedBox(
              width: 110,
              child: TextFormField(
                initialValue: '${fault['exit_code'] ?? 1}',
                decoration: const InputDecoration(labelText: 'Exit code'),
                keyboardType: TextInputType.number,
                onChanged: (value) {
                  final parsed = int.tryParse(value);
                  if (parsed != null) {
                    fault['exit_code'] = parsed;
                    onChanged();
                  }
                },
              ),
            ),
            const SizedBox(width: 8),
            SizedBox(
              width: 120,
              child: SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('Once'),
                value: fault['once'] as bool? ?? true,
                onChanged: (value) {
                  fault['once'] = value;
                  onChanged();
                },
              ),
            ),
          ],
          const Spacer(),
          IconButton(
            tooltip: 'Remove fault',
            onPressed: onRemove,
            icon: const Icon(Icons.remove_circle_outline, size: 18),
          ),
        ],
      ),
    );
  }

  Widget _number(String key, String label) => Padding(
    padding: const EdgeInsets.only(right: 8),
    child: SizedBox(
      width: 120,
      child: TextFormField(
        initialValue: '${fault[key] ?? ''}',
        decoration: InputDecoration(labelText: label),
        keyboardType: TextInputType.number,
        onChanged: (value) {
          final parsed = int.tryParse(value);
          if (parsed != null) {
            fault[key] = parsed;
            onChanged();
          }
        },
      ),
    ),
  );
}

class _SaveIndicator extends StatelessWidget {
  const _SaveIndicator({
    required this.saving,
    required this.label,
    required this.dirty,
  });

  final bool saving;
  final String label;
  final bool dirty;

  @override
  Widget build(BuildContext context) {
    final color = saving
        ? context.statusColors.info
        : dirty
        ? context.statusColors.warning
        : context.statusColors.healthy;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (saving)
          const SizedBox(
            width: 12,
            height: 12,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          Icon(
            dirty ? Icons.edit_outlined : Icons.check_circle_outline,
            size: 14,
            color: color,
          ),
        const SizedBox(width: 6),
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(color: color),
        ),
      ],
    );
  }
}
