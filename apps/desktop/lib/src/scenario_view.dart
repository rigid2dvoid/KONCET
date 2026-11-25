import 'dart:convert';

/// Read-only projection of a stored scenario for visual explanation.
///
/// The simulation engine remains authoritative. These values are only used to
/// label the operations board and route sketch.
class ScenarioView {
  const ScenarioView({
    required this.profile,
    required this.serviceName,
    required this.servicePort,
    required this.proxyEnabled,
    required this.proxyHost,
    required this.proxyPath,
    required this.proxyListenerPort,
    required this.proxyTargetPort,
    required this.firewallDefaultPolicy,
    required this.firewallAllowCount,
    required this.firewallDenyCount,
    required this.monitoringIntervalS,
    required this.maxSimSeconds,
  });

  final String profile;
  final String serviceName;
  final int servicePort;
  final bool proxyEnabled;
  final String proxyHost;
  final String proxyPath;
  final int proxyListenerPort;
  final int proxyTargetPort;
  final String firewallDefaultPolicy;
  final int firewallAllowCount;
  final int firewallDenyCount;
  final int monitoringIntervalS;
  final int maxSimSeconds;

  static ScenarioView? parse(String json) {
    try {
      final value = jsonDecode(json);
      if (value is! Map<String, dynamic>) return null;
      final build = _map(value['build']);
      final service = _map(value['service']);
      final proxy = _map(value['proxy']);
      final firewall = _map(value['firewall']);
      final monitoring = _map(value['monitoring']);
      final limits = _map(value['limits']);
      final rules = firewall['rules'];
      final ruleList = rules is List ? rules : const [];
      var allow = 0;
      var deny = 0;
      for (final rule in ruleList) {
        if (rule is Map<String, dynamic>) {
          if (rule['action'] == 'allow') allow++;
          if (rule['action'] == 'deny') deny++;
        }
      }
      return ScenarioView(
        profile: '${build['profile'] ?? 'static-web'}',
        serviceName: '${service['name'] ?? 'app'}',
        servicePort: _int(service['port'], 3000),
        proxyEnabled: proxy['enabled'] == true,
        proxyHost: '${proxy['host'] ?? 'app.koncet.local'}',
        proxyPath: '${proxy['path'] ?? '/'}',
        proxyListenerPort: _int(proxy['listener_port'], 8080),
        proxyTargetPort: _int(proxy['target_port'], 3000),
        firewallDefaultPolicy: '${firewall['default_policy'] ?? 'deny'}',
        firewallAllowCount: allow,
        firewallDenyCount: deny,
        monitoringIntervalS: _int(monitoring['sample_interval_s'], 5),
        maxSimSeconds: _int(limits['max_sim_seconds'], 3600),
      );
    } on FormatException {
      return null;
    } on TypeError {
      return null;
    }
  }

  static Map<String, dynamic> _map(Object? value) =>
      value is Map<String, dynamic> ? value : const {};

  static int _int(Object? value, int fallback) =>
      value is int ? value : fallback;
}
