// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'lib.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models');

/// @nodoc
mixin _$AuthHeaderFormat {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() bearer,
    required TResult Function(String field0) apiKey,
    required TResult Function() none,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? bearer,
    TResult? Function(String field0)? apiKey,
    TResult? Function()? none,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? bearer,
    TResult Function(String field0)? apiKey,
    TResult Function()? none,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(AuthHeaderFormat_Bearer value) bearer,
    required TResult Function(AuthHeaderFormat_ApiKey value) apiKey,
    required TResult Function(AuthHeaderFormat_None value) none,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult? Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult? Function(AuthHeaderFormat_None value)? none,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult Function(AuthHeaderFormat_None value)? none,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $AuthHeaderFormatCopyWith<$Res> {
  factory $AuthHeaderFormatCopyWith(
          AuthHeaderFormat value, $Res Function(AuthHeaderFormat) then) =
      _$AuthHeaderFormatCopyWithImpl<$Res, AuthHeaderFormat>;
}

/// @nodoc
class _$AuthHeaderFormatCopyWithImpl<$Res, $Val extends AuthHeaderFormat>
    implements $AuthHeaderFormatCopyWith<$Res> {
  _$AuthHeaderFormatCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$AuthHeaderFormat_BearerImplCopyWith<$Res> {
  factory _$$AuthHeaderFormat_BearerImplCopyWith(
          _$AuthHeaderFormat_BearerImpl value,
          $Res Function(_$AuthHeaderFormat_BearerImpl) then) =
      __$$AuthHeaderFormat_BearerImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$AuthHeaderFormat_BearerImplCopyWithImpl<$Res>
    extends _$AuthHeaderFormatCopyWithImpl<$Res, _$AuthHeaderFormat_BearerImpl>
    implements _$$AuthHeaderFormat_BearerImplCopyWith<$Res> {
  __$$AuthHeaderFormat_BearerImplCopyWithImpl(
      _$AuthHeaderFormat_BearerImpl _value,
      $Res Function(_$AuthHeaderFormat_BearerImpl) _then)
      : super(_value, _then);

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$AuthHeaderFormat_BearerImpl extends AuthHeaderFormat_Bearer {
  const _$AuthHeaderFormat_BearerImpl() : super._();

  @override
  String toString() {
    return 'AuthHeaderFormat.bearer()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$AuthHeaderFormat_BearerImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() bearer,
    required TResult Function(String field0) apiKey,
    required TResult Function() none,
  }) {
    return bearer();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? bearer,
    TResult? Function(String field0)? apiKey,
    TResult? Function()? none,
  }) {
    return bearer?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? bearer,
    TResult Function(String field0)? apiKey,
    TResult Function()? none,
    required TResult orElse(),
  }) {
    if (bearer != null) {
      return bearer();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(AuthHeaderFormat_Bearer value) bearer,
    required TResult Function(AuthHeaderFormat_ApiKey value) apiKey,
    required TResult Function(AuthHeaderFormat_None value) none,
  }) {
    return bearer(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult? Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult? Function(AuthHeaderFormat_None value)? none,
  }) {
    return bearer?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult Function(AuthHeaderFormat_None value)? none,
    required TResult orElse(),
  }) {
    if (bearer != null) {
      return bearer(this);
    }
    return orElse();
  }
}

abstract class AuthHeaderFormat_Bearer extends AuthHeaderFormat {
  const factory AuthHeaderFormat_Bearer() = _$AuthHeaderFormat_BearerImpl;
  const AuthHeaderFormat_Bearer._() : super._();
}

/// @nodoc
abstract class _$$AuthHeaderFormat_ApiKeyImplCopyWith<$Res> {
  factory _$$AuthHeaderFormat_ApiKeyImplCopyWith(
          _$AuthHeaderFormat_ApiKeyImpl value,
          $Res Function(_$AuthHeaderFormat_ApiKeyImpl) then) =
      __$$AuthHeaderFormat_ApiKeyImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$AuthHeaderFormat_ApiKeyImplCopyWithImpl<$Res>
    extends _$AuthHeaderFormatCopyWithImpl<$Res, _$AuthHeaderFormat_ApiKeyImpl>
    implements _$$AuthHeaderFormat_ApiKeyImplCopyWith<$Res> {
  __$$AuthHeaderFormat_ApiKeyImplCopyWithImpl(
      _$AuthHeaderFormat_ApiKeyImpl _value,
      $Res Function(_$AuthHeaderFormat_ApiKeyImpl) _then)
      : super(_value, _then);

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$AuthHeaderFormat_ApiKeyImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$AuthHeaderFormat_ApiKeyImpl extends AuthHeaderFormat_ApiKey {
  const _$AuthHeaderFormat_ApiKeyImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'AuthHeaderFormat.apiKey(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$AuthHeaderFormat_ApiKeyImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$AuthHeaderFormat_ApiKeyImplCopyWith<_$AuthHeaderFormat_ApiKeyImpl>
      get copyWith => __$$AuthHeaderFormat_ApiKeyImplCopyWithImpl<
          _$AuthHeaderFormat_ApiKeyImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() bearer,
    required TResult Function(String field0) apiKey,
    required TResult Function() none,
  }) {
    return apiKey(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? bearer,
    TResult? Function(String field0)? apiKey,
    TResult? Function()? none,
  }) {
    return apiKey?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? bearer,
    TResult Function(String field0)? apiKey,
    TResult Function()? none,
    required TResult orElse(),
  }) {
    if (apiKey != null) {
      return apiKey(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(AuthHeaderFormat_Bearer value) bearer,
    required TResult Function(AuthHeaderFormat_ApiKey value) apiKey,
    required TResult Function(AuthHeaderFormat_None value) none,
  }) {
    return apiKey(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult? Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult? Function(AuthHeaderFormat_None value)? none,
  }) {
    return apiKey?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult Function(AuthHeaderFormat_None value)? none,
    required TResult orElse(),
  }) {
    if (apiKey != null) {
      return apiKey(this);
    }
    return orElse();
  }
}

abstract class AuthHeaderFormat_ApiKey extends AuthHeaderFormat {
  const factory AuthHeaderFormat_ApiKey({required final String field0}) =
      _$AuthHeaderFormat_ApiKeyImpl;
  const AuthHeaderFormat_ApiKey._() : super._();

  String get field0;

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$AuthHeaderFormat_ApiKeyImplCopyWith<_$AuthHeaderFormat_ApiKeyImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$AuthHeaderFormat_NoneImplCopyWith<$Res> {
  factory _$$AuthHeaderFormat_NoneImplCopyWith(
          _$AuthHeaderFormat_NoneImpl value,
          $Res Function(_$AuthHeaderFormat_NoneImpl) then) =
      __$$AuthHeaderFormat_NoneImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$AuthHeaderFormat_NoneImplCopyWithImpl<$Res>
    extends _$AuthHeaderFormatCopyWithImpl<$Res, _$AuthHeaderFormat_NoneImpl>
    implements _$$AuthHeaderFormat_NoneImplCopyWith<$Res> {
  __$$AuthHeaderFormat_NoneImplCopyWithImpl(_$AuthHeaderFormat_NoneImpl _value,
      $Res Function(_$AuthHeaderFormat_NoneImpl) _then)
      : super(_value, _then);

  /// Create a copy of AuthHeaderFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$AuthHeaderFormat_NoneImpl extends AuthHeaderFormat_None {
  const _$AuthHeaderFormat_NoneImpl() : super._();

  @override
  String toString() {
    return 'AuthHeaderFormat.none()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$AuthHeaderFormat_NoneImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() bearer,
    required TResult Function(String field0) apiKey,
    required TResult Function() none,
  }) {
    return none();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? bearer,
    TResult? Function(String field0)? apiKey,
    TResult? Function()? none,
  }) {
    return none?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? bearer,
    TResult Function(String field0)? apiKey,
    TResult Function()? none,
    required TResult orElse(),
  }) {
    if (none != null) {
      return none();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(AuthHeaderFormat_Bearer value) bearer,
    required TResult Function(AuthHeaderFormat_ApiKey value) apiKey,
    required TResult Function(AuthHeaderFormat_None value) none,
  }) {
    return none(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult? Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult? Function(AuthHeaderFormat_None value)? none,
  }) {
    return none?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(AuthHeaderFormat_Bearer value)? bearer,
    TResult Function(AuthHeaderFormat_ApiKey value)? apiKey,
    TResult Function(AuthHeaderFormat_None value)? none,
    required TResult orElse(),
  }) {
    if (none != null) {
      return none(this);
    }
    return orElse();
  }
}

abstract class AuthHeaderFormat_None extends AuthHeaderFormat {
  const factory AuthHeaderFormat_None() = _$AuthHeaderFormat_NoneImpl;
  const AuthHeaderFormat_None._() : super._();
}
