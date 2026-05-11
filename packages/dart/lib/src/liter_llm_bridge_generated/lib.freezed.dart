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

/// @nodoc
mixin _$ContentPart {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) text,
    required TResult Function(ImageUrl imageUrl) imageUrl,
    required TResult Function(DocumentContent document) document,
    required TResult Function(AudioContent inputAudio) inputAudio,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? text,
    TResult? Function(ImageUrl imageUrl)? imageUrl,
    TResult? Function(DocumentContent document)? document,
    TResult? Function(AudioContent inputAudio)? inputAudio,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? text,
    TResult Function(ImageUrl imageUrl)? imageUrl,
    TResult Function(DocumentContent document)? document,
    TResult Function(AudioContent inputAudio)? inputAudio,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentPart_Text value) text,
    required TResult Function(ContentPart_ImageUrl value) imageUrl,
    required TResult Function(ContentPart_Document value) document,
    required TResult Function(ContentPart_InputAudio value) inputAudio,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentPart_Text value)? text,
    TResult? Function(ContentPart_ImageUrl value)? imageUrl,
    TResult? Function(ContentPart_Document value)? document,
    TResult? Function(ContentPart_InputAudio value)? inputAudio,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentPart_Text value)? text,
    TResult Function(ContentPart_ImageUrl value)? imageUrl,
    TResult Function(ContentPart_Document value)? document,
    TResult Function(ContentPart_InputAudio value)? inputAudio,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ContentPartCopyWith<$Res> {
  factory $ContentPartCopyWith(
          ContentPart value, $Res Function(ContentPart) then) =
      _$ContentPartCopyWithImpl<$Res, ContentPart>;
}

/// @nodoc
class _$ContentPartCopyWithImpl<$Res, $Val extends ContentPart>
    implements $ContentPartCopyWith<$Res> {
  _$ContentPartCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$ContentPart_TextImplCopyWith<$Res> {
  factory _$$ContentPart_TextImplCopyWith(_$ContentPart_TextImpl value,
          $Res Function(_$ContentPart_TextImpl) then) =
      __$$ContentPart_TextImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String text});
}

/// @nodoc
class __$$ContentPart_TextImplCopyWithImpl<$Res>
    extends _$ContentPartCopyWithImpl<$Res, _$ContentPart_TextImpl>
    implements _$$ContentPart_TextImplCopyWith<$Res> {
  __$$ContentPart_TextImplCopyWithImpl(_$ContentPart_TextImpl _value,
      $Res Function(_$ContentPart_TextImpl) _then)
      : super(_value, _then);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? text = null,
  }) {
    return _then(_$ContentPart_TextImpl(
      text: null == text
          ? _value.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$ContentPart_TextImpl extends ContentPart_Text {
  const _$ContentPart_TextImpl({required this.text}) : super._();

  @override
  final String text;

  @override
  String toString() {
    return 'ContentPart.text(text: $text)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentPart_TextImpl &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentPart_TextImplCopyWith<_$ContentPart_TextImpl> get copyWith =>
      __$$ContentPart_TextImplCopyWithImpl<_$ContentPart_TextImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) text,
    required TResult Function(ImageUrl imageUrl) imageUrl,
    required TResult Function(DocumentContent document) document,
    required TResult Function(AudioContent inputAudio) inputAudio,
  }) {
    return text(this.text);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? text,
    TResult? Function(ImageUrl imageUrl)? imageUrl,
    TResult? Function(DocumentContent document)? document,
    TResult? Function(AudioContent inputAudio)? inputAudio,
  }) {
    return text?.call(this.text);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? text,
    TResult Function(ImageUrl imageUrl)? imageUrl,
    TResult Function(DocumentContent document)? document,
    TResult Function(AudioContent inputAudio)? inputAudio,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this.text);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentPart_Text value) text,
    required TResult Function(ContentPart_ImageUrl value) imageUrl,
    required TResult Function(ContentPart_Document value) document,
    required TResult Function(ContentPart_InputAudio value) inputAudio,
  }) {
    return text(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentPart_Text value)? text,
    TResult? Function(ContentPart_ImageUrl value)? imageUrl,
    TResult? Function(ContentPart_Document value)? document,
    TResult? Function(ContentPart_InputAudio value)? inputAudio,
  }) {
    return text?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentPart_Text value)? text,
    TResult Function(ContentPart_ImageUrl value)? imageUrl,
    TResult Function(ContentPart_Document value)? document,
    TResult Function(ContentPart_InputAudio value)? inputAudio,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this);
    }
    return orElse();
  }
}

abstract class ContentPart_Text extends ContentPart {
  const factory ContentPart_Text({required final String text}) =
      _$ContentPart_TextImpl;
  const ContentPart_Text._() : super._();

  String get text;

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ContentPart_TextImplCopyWith<_$ContentPart_TextImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ContentPart_ImageUrlImplCopyWith<$Res> {
  factory _$$ContentPart_ImageUrlImplCopyWith(_$ContentPart_ImageUrlImpl value,
          $Res Function(_$ContentPart_ImageUrlImpl) then) =
      __$$ContentPart_ImageUrlImplCopyWithImpl<$Res>;
  @useResult
  $Res call({ImageUrl imageUrl});
}

/// @nodoc
class __$$ContentPart_ImageUrlImplCopyWithImpl<$Res>
    extends _$ContentPartCopyWithImpl<$Res, _$ContentPart_ImageUrlImpl>
    implements _$$ContentPart_ImageUrlImplCopyWith<$Res> {
  __$$ContentPart_ImageUrlImplCopyWithImpl(_$ContentPart_ImageUrlImpl _value,
      $Res Function(_$ContentPart_ImageUrlImpl) _then)
      : super(_value, _then);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? imageUrl = null,
  }) {
    return _then(_$ContentPart_ImageUrlImpl(
      imageUrl: null == imageUrl
          ? _value.imageUrl
          : imageUrl // ignore: cast_nullable_to_non_nullable
              as ImageUrl,
    ));
  }
}

/// @nodoc

class _$ContentPart_ImageUrlImpl extends ContentPart_ImageUrl {
  const _$ContentPart_ImageUrlImpl({required this.imageUrl}) : super._();

  @override
  final ImageUrl imageUrl;

  @override
  String toString() {
    return 'ContentPart.imageUrl(imageUrl: $imageUrl)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentPart_ImageUrlImpl &&
            (identical(other.imageUrl, imageUrl) ||
                other.imageUrl == imageUrl));
  }

  @override
  int get hashCode => Object.hash(runtimeType, imageUrl);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentPart_ImageUrlImplCopyWith<_$ContentPart_ImageUrlImpl>
      get copyWith =>
          __$$ContentPart_ImageUrlImplCopyWithImpl<_$ContentPart_ImageUrlImpl>(
              this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) text,
    required TResult Function(ImageUrl imageUrl) imageUrl,
    required TResult Function(DocumentContent document) document,
    required TResult Function(AudioContent inputAudio) inputAudio,
  }) {
    return imageUrl(this.imageUrl);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? text,
    TResult? Function(ImageUrl imageUrl)? imageUrl,
    TResult? Function(DocumentContent document)? document,
    TResult? Function(AudioContent inputAudio)? inputAudio,
  }) {
    return imageUrl?.call(this.imageUrl);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? text,
    TResult Function(ImageUrl imageUrl)? imageUrl,
    TResult Function(DocumentContent document)? document,
    TResult Function(AudioContent inputAudio)? inputAudio,
    required TResult orElse(),
  }) {
    if (imageUrl != null) {
      return imageUrl(this.imageUrl);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentPart_Text value) text,
    required TResult Function(ContentPart_ImageUrl value) imageUrl,
    required TResult Function(ContentPart_Document value) document,
    required TResult Function(ContentPart_InputAudio value) inputAudio,
  }) {
    return imageUrl(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentPart_Text value)? text,
    TResult? Function(ContentPart_ImageUrl value)? imageUrl,
    TResult? Function(ContentPart_Document value)? document,
    TResult? Function(ContentPart_InputAudio value)? inputAudio,
  }) {
    return imageUrl?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentPart_Text value)? text,
    TResult Function(ContentPart_ImageUrl value)? imageUrl,
    TResult Function(ContentPart_Document value)? document,
    TResult Function(ContentPart_InputAudio value)? inputAudio,
    required TResult orElse(),
  }) {
    if (imageUrl != null) {
      return imageUrl(this);
    }
    return orElse();
  }
}

abstract class ContentPart_ImageUrl extends ContentPart {
  const factory ContentPart_ImageUrl({required final ImageUrl imageUrl}) =
      _$ContentPart_ImageUrlImpl;
  const ContentPart_ImageUrl._() : super._();

  ImageUrl get imageUrl;

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ContentPart_ImageUrlImplCopyWith<_$ContentPart_ImageUrlImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ContentPart_DocumentImplCopyWith<$Res> {
  factory _$$ContentPart_DocumentImplCopyWith(_$ContentPart_DocumentImpl value,
          $Res Function(_$ContentPart_DocumentImpl) then) =
      __$$ContentPart_DocumentImplCopyWithImpl<$Res>;
  @useResult
  $Res call({DocumentContent document});
}

/// @nodoc
class __$$ContentPart_DocumentImplCopyWithImpl<$Res>
    extends _$ContentPartCopyWithImpl<$Res, _$ContentPart_DocumentImpl>
    implements _$$ContentPart_DocumentImplCopyWith<$Res> {
  __$$ContentPart_DocumentImplCopyWithImpl(_$ContentPart_DocumentImpl _value,
      $Res Function(_$ContentPart_DocumentImpl) _then)
      : super(_value, _then);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? document = null,
  }) {
    return _then(_$ContentPart_DocumentImpl(
      document: null == document
          ? _value.document
          : document // ignore: cast_nullable_to_non_nullable
              as DocumentContent,
    ));
  }
}

/// @nodoc

class _$ContentPart_DocumentImpl extends ContentPart_Document {
  const _$ContentPart_DocumentImpl({required this.document}) : super._();

  @override
  final DocumentContent document;

  @override
  String toString() {
    return 'ContentPart.document(document: $document)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentPart_DocumentImpl &&
            (identical(other.document, document) ||
                other.document == document));
  }

  @override
  int get hashCode => Object.hash(runtimeType, document);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentPart_DocumentImplCopyWith<_$ContentPart_DocumentImpl>
      get copyWith =>
          __$$ContentPart_DocumentImplCopyWithImpl<_$ContentPart_DocumentImpl>(
              this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) text,
    required TResult Function(ImageUrl imageUrl) imageUrl,
    required TResult Function(DocumentContent document) document,
    required TResult Function(AudioContent inputAudio) inputAudio,
  }) {
    return document(this.document);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? text,
    TResult? Function(ImageUrl imageUrl)? imageUrl,
    TResult? Function(DocumentContent document)? document,
    TResult? Function(AudioContent inputAudio)? inputAudio,
  }) {
    return document?.call(this.document);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? text,
    TResult Function(ImageUrl imageUrl)? imageUrl,
    TResult Function(DocumentContent document)? document,
    TResult Function(AudioContent inputAudio)? inputAudio,
    required TResult orElse(),
  }) {
    if (document != null) {
      return document(this.document);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentPart_Text value) text,
    required TResult Function(ContentPart_ImageUrl value) imageUrl,
    required TResult Function(ContentPart_Document value) document,
    required TResult Function(ContentPart_InputAudio value) inputAudio,
  }) {
    return document(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentPart_Text value)? text,
    TResult? Function(ContentPart_ImageUrl value)? imageUrl,
    TResult? Function(ContentPart_Document value)? document,
    TResult? Function(ContentPart_InputAudio value)? inputAudio,
  }) {
    return document?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentPart_Text value)? text,
    TResult Function(ContentPart_ImageUrl value)? imageUrl,
    TResult Function(ContentPart_Document value)? document,
    TResult Function(ContentPart_InputAudio value)? inputAudio,
    required TResult orElse(),
  }) {
    if (document != null) {
      return document(this);
    }
    return orElse();
  }
}

abstract class ContentPart_Document extends ContentPart {
  const factory ContentPart_Document(
      {required final DocumentContent document}) = _$ContentPart_DocumentImpl;
  const ContentPart_Document._() : super._();

  DocumentContent get document;

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ContentPart_DocumentImplCopyWith<_$ContentPart_DocumentImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ContentPart_InputAudioImplCopyWith<$Res> {
  factory _$$ContentPart_InputAudioImplCopyWith(
          _$ContentPart_InputAudioImpl value,
          $Res Function(_$ContentPart_InputAudioImpl) then) =
      __$$ContentPart_InputAudioImplCopyWithImpl<$Res>;
  @useResult
  $Res call({AudioContent inputAudio});
}

/// @nodoc
class __$$ContentPart_InputAudioImplCopyWithImpl<$Res>
    extends _$ContentPartCopyWithImpl<$Res, _$ContentPart_InputAudioImpl>
    implements _$$ContentPart_InputAudioImplCopyWith<$Res> {
  __$$ContentPart_InputAudioImplCopyWithImpl(
      _$ContentPart_InputAudioImpl _value,
      $Res Function(_$ContentPart_InputAudioImpl) _then)
      : super(_value, _then);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? inputAudio = null,
  }) {
    return _then(_$ContentPart_InputAudioImpl(
      inputAudio: null == inputAudio
          ? _value.inputAudio
          : inputAudio // ignore: cast_nullable_to_non_nullable
              as AudioContent,
    ));
  }
}

/// @nodoc

class _$ContentPart_InputAudioImpl extends ContentPart_InputAudio {
  const _$ContentPart_InputAudioImpl({required this.inputAudio}) : super._();

  @override
  final AudioContent inputAudio;

  @override
  String toString() {
    return 'ContentPart.inputAudio(inputAudio: $inputAudio)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentPart_InputAudioImpl &&
            (identical(other.inputAudio, inputAudio) ||
                other.inputAudio == inputAudio));
  }

  @override
  int get hashCode => Object.hash(runtimeType, inputAudio);

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentPart_InputAudioImplCopyWith<_$ContentPart_InputAudioImpl>
      get copyWith => __$$ContentPart_InputAudioImplCopyWithImpl<
          _$ContentPart_InputAudioImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) text,
    required TResult Function(ImageUrl imageUrl) imageUrl,
    required TResult Function(DocumentContent document) document,
    required TResult Function(AudioContent inputAudio) inputAudio,
  }) {
    return inputAudio(this.inputAudio);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? text,
    TResult? Function(ImageUrl imageUrl)? imageUrl,
    TResult? Function(DocumentContent document)? document,
    TResult? Function(AudioContent inputAudio)? inputAudio,
  }) {
    return inputAudio?.call(this.inputAudio);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? text,
    TResult Function(ImageUrl imageUrl)? imageUrl,
    TResult Function(DocumentContent document)? document,
    TResult Function(AudioContent inputAudio)? inputAudio,
    required TResult orElse(),
  }) {
    if (inputAudio != null) {
      return inputAudio(this.inputAudio);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentPart_Text value) text,
    required TResult Function(ContentPart_ImageUrl value) imageUrl,
    required TResult Function(ContentPart_Document value) document,
    required TResult Function(ContentPart_InputAudio value) inputAudio,
  }) {
    return inputAudio(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentPart_Text value)? text,
    TResult? Function(ContentPart_ImageUrl value)? imageUrl,
    TResult? Function(ContentPart_Document value)? document,
    TResult? Function(ContentPart_InputAudio value)? inputAudio,
  }) {
    return inputAudio?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentPart_Text value)? text,
    TResult Function(ContentPart_ImageUrl value)? imageUrl,
    TResult Function(ContentPart_Document value)? document,
    TResult Function(ContentPart_InputAudio value)? inputAudio,
    required TResult orElse(),
  }) {
    if (inputAudio != null) {
      return inputAudio(this);
    }
    return orElse();
  }
}

abstract class ContentPart_InputAudio extends ContentPart {
  const factory ContentPart_InputAudio(
      {required final AudioContent inputAudio}) = _$ContentPart_InputAudioImpl;
  const ContentPart_InputAudio._() : super._();

  AudioContent get inputAudio;

  /// Create a copy of ContentPart
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ContentPart_InputAudioImplCopyWith<_$ContentPart_InputAudioImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$EmbeddingInput {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(EmbeddingInput_Single value) single,
    required TResult Function(EmbeddingInput_Multiple value) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(EmbeddingInput_Single value)? single,
    TResult? Function(EmbeddingInput_Multiple value)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(EmbeddingInput_Single value)? single,
    TResult Function(EmbeddingInput_Multiple value)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $EmbeddingInputCopyWith<$Res> {
  factory $EmbeddingInputCopyWith(
          EmbeddingInput value, $Res Function(EmbeddingInput) then) =
      _$EmbeddingInputCopyWithImpl<$Res, EmbeddingInput>;
}

/// @nodoc
class _$EmbeddingInputCopyWithImpl<$Res, $Val extends EmbeddingInput>
    implements $EmbeddingInputCopyWith<$Res> {
  _$EmbeddingInputCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$EmbeddingInput_SingleImplCopyWith<$Res> {
  factory _$$EmbeddingInput_SingleImplCopyWith(
          _$EmbeddingInput_SingleImpl value,
          $Res Function(_$EmbeddingInput_SingleImpl) then) =
      __$$EmbeddingInput_SingleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$EmbeddingInput_SingleImplCopyWithImpl<$Res>
    extends _$EmbeddingInputCopyWithImpl<$Res, _$EmbeddingInput_SingleImpl>
    implements _$$EmbeddingInput_SingleImplCopyWith<$Res> {
  __$$EmbeddingInput_SingleImplCopyWithImpl(_$EmbeddingInput_SingleImpl _value,
      $Res Function(_$EmbeddingInput_SingleImpl) _then)
      : super(_value, _then);

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$EmbeddingInput_SingleImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$EmbeddingInput_SingleImpl extends EmbeddingInput_Single {
  const _$EmbeddingInput_SingleImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'EmbeddingInput.single(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$EmbeddingInput_SingleImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$EmbeddingInput_SingleImplCopyWith<_$EmbeddingInput_SingleImpl>
      get copyWith => __$$EmbeddingInput_SingleImplCopyWithImpl<
          _$EmbeddingInput_SingleImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return single(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return single?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(EmbeddingInput_Single value) single,
    required TResult Function(EmbeddingInput_Multiple value) multiple,
  }) {
    return single(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(EmbeddingInput_Single value)? single,
    TResult? Function(EmbeddingInput_Multiple value)? multiple,
  }) {
    return single?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(EmbeddingInput_Single value)? single,
    TResult Function(EmbeddingInput_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(this);
    }
    return orElse();
  }
}

abstract class EmbeddingInput_Single extends EmbeddingInput {
  const factory EmbeddingInput_Single({required final String field0}) =
      _$EmbeddingInput_SingleImpl;
  const EmbeddingInput_Single._() : super._();

  @override
  String get field0;

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$EmbeddingInput_SingleImplCopyWith<_$EmbeddingInput_SingleImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$EmbeddingInput_MultipleImplCopyWith<$Res> {
  factory _$$EmbeddingInput_MultipleImplCopyWith(
          _$EmbeddingInput_MultipleImpl value,
          $Res Function(_$EmbeddingInput_MultipleImpl) then) =
      __$$EmbeddingInput_MultipleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<String> field0});
}

/// @nodoc
class __$$EmbeddingInput_MultipleImplCopyWithImpl<$Res>
    extends _$EmbeddingInputCopyWithImpl<$Res, _$EmbeddingInput_MultipleImpl>
    implements _$$EmbeddingInput_MultipleImplCopyWith<$Res> {
  __$$EmbeddingInput_MultipleImplCopyWithImpl(
      _$EmbeddingInput_MultipleImpl _value,
      $Res Function(_$EmbeddingInput_MultipleImpl) _then)
      : super(_value, _then);

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$EmbeddingInput_MultipleImpl(
      field0: null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<String>,
    ));
  }
}

/// @nodoc

class _$EmbeddingInput_MultipleImpl extends EmbeddingInput_Multiple {
  const _$EmbeddingInput_MultipleImpl({required final List<String> field0})
      : _field0 = field0,
        super._();

  final List<String> _field0;
  @override
  List<String> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'EmbeddingInput.multiple(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$EmbeddingInput_MultipleImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$EmbeddingInput_MultipleImplCopyWith<_$EmbeddingInput_MultipleImpl>
      get copyWith => __$$EmbeddingInput_MultipleImplCopyWithImpl<
          _$EmbeddingInput_MultipleImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return multiple(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return multiple?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(EmbeddingInput_Single value) single,
    required TResult Function(EmbeddingInput_Multiple value) multiple,
  }) {
    return multiple(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(EmbeddingInput_Single value)? single,
    TResult? Function(EmbeddingInput_Multiple value)? multiple,
  }) {
    return multiple?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(EmbeddingInput_Single value)? single,
    TResult Function(EmbeddingInput_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(this);
    }
    return orElse();
  }
}

abstract class EmbeddingInput_Multiple extends EmbeddingInput {
  const factory EmbeddingInput_Multiple({required final List<String> field0}) =
      _$EmbeddingInput_MultipleImpl;
  const EmbeddingInput_Multiple._() : super._();

  @override
  List<String> get field0;

  /// Create a copy of EmbeddingInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$EmbeddingInput_MultipleImplCopyWith<_$EmbeddingInput_MultipleImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$Message {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $MessageCopyWith<$Res> {
  factory $MessageCopyWith(Message value, $Res Function(Message) then) =
      _$MessageCopyWithImpl<$Res, Message>;
}

/// @nodoc
class _$MessageCopyWithImpl<$Res, $Val extends Message>
    implements $MessageCopyWith<$Res> {
  _$MessageCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$Message_SystemImplCopyWith<$Res> {
  factory _$$Message_SystemImplCopyWith(_$Message_SystemImpl value,
          $Res Function(_$Message_SystemImpl) then) =
      __$$Message_SystemImplCopyWithImpl<$Res>;
  @useResult
  $Res call({SystemMessage field0});
}

/// @nodoc
class __$$Message_SystemImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_SystemImpl>
    implements _$$Message_SystemImplCopyWith<$Res> {
  __$$Message_SystemImplCopyWithImpl(
      _$Message_SystemImpl _value, $Res Function(_$Message_SystemImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_SystemImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as SystemMessage,
    ));
  }
}

/// @nodoc

class _$Message_SystemImpl extends Message_System {
  const _$Message_SystemImpl({required this.field0}) : super._();

  @override
  final SystemMessage field0;

  @override
  String toString() {
    return 'Message.system(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_SystemImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_SystemImplCopyWith<_$Message_SystemImpl> get copyWith =>
      __$$Message_SystemImplCopyWithImpl<_$Message_SystemImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return system(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return system?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (system != null) {
      return system(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return system(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return system?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (system != null) {
      return system(this);
    }
    return orElse();
  }
}

abstract class Message_System extends Message {
  const factory Message_System({required final SystemMessage field0}) =
      _$Message_SystemImpl;
  const Message_System._() : super._();

  @override
  SystemMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_SystemImplCopyWith<_$Message_SystemImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$Message_UserImplCopyWith<$Res> {
  factory _$$Message_UserImplCopyWith(
          _$Message_UserImpl value, $Res Function(_$Message_UserImpl) then) =
      __$$Message_UserImplCopyWithImpl<$Res>;
  @useResult
  $Res call({UserMessage field0});
}

/// @nodoc
class __$$Message_UserImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_UserImpl>
    implements _$$Message_UserImplCopyWith<$Res> {
  __$$Message_UserImplCopyWithImpl(
      _$Message_UserImpl _value, $Res Function(_$Message_UserImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_UserImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as UserMessage,
    ));
  }
}

/// @nodoc

class _$Message_UserImpl extends Message_User {
  const _$Message_UserImpl({required this.field0}) : super._();

  @override
  final UserMessage field0;

  @override
  String toString() {
    return 'Message.user(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_UserImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_UserImplCopyWith<_$Message_UserImpl> get copyWith =>
      __$$Message_UserImplCopyWithImpl<_$Message_UserImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return user(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return user?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (user != null) {
      return user(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return user(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return user?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (user != null) {
      return user(this);
    }
    return orElse();
  }
}

abstract class Message_User extends Message {
  const factory Message_User({required final UserMessage field0}) =
      _$Message_UserImpl;
  const Message_User._() : super._();

  @override
  UserMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_UserImplCopyWith<_$Message_UserImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$Message_AssistantImplCopyWith<$Res> {
  factory _$$Message_AssistantImplCopyWith(_$Message_AssistantImpl value,
          $Res Function(_$Message_AssistantImpl) then) =
      __$$Message_AssistantImplCopyWithImpl<$Res>;
  @useResult
  $Res call({AssistantMessage field0});
}

/// @nodoc
class __$$Message_AssistantImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_AssistantImpl>
    implements _$$Message_AssistantImplCopyWith<$Res> {
  __$$Message_AssistantImplCopyWithImpl(_$Message_AssistantImpl _value,
      $Res Function(_$Message_AssistantImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_AssistantImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as AssistantMessage,
    ));
  }
}

/// @nodoc

class _$Message_AssistantImpl extends Message_Assistant {
  const _$Message_AssistantImpl({required this.field0}) : super._();

  @override
  final AssistantMessage field0;

  @override
  String toString() {
    return 'Message.assistant(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_AssistantImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_AssistantImplCopyWith<_$Message_AssistantImpl> get copyWith =>
      __$$Message_AssistantImplCopyWithImpl<_$Message_AssistantImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return assistant(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return assistant?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (assistant != null) {
      return assistant(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return assistant(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return assistant?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (assistant != null) {
      return assistant(this);
    }
    return orElse();
  }
}

abstract class Message_Assistant extends Message {
  const factory Message_Assistant({required final AssistantMessage field0}) =
      _$Message_AssistantImpl;
  const Message_Assistant._() : super._();

  @override
  AssistantMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_AssistantImplCopyWith<_$Message_AssistantImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$Message_ToolImplCopyWith<$Res> {
  factory _$$Message_ToolImplCopyWith(
          _$Message_ToolImpl value, $Res Function(_$Message_ToolImpl) then) =
      __$$Message_ToolImplCopyWithImpl<$Res>;
  @useResult
  $Res call({ToolMessage field0});
}

/// @nodoc
class __$$Message_ToolImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_ToolImpl>
    implements _$$Message_ToolImplCopyWith<$Res> {
  __$$Message_ToolImplCopyWithImpl(
      _$Message_ToolImpl _value, $Res Function(_$Message_ToolImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_ToolImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ToolMessage,
    ));
  }
}

/// @nodoc

class _$Message_ToolImpl extends Message_Tool {
  const _$Message_ToolImpl({required this.field0}) : super._();

  @override
  final ToolMessage field0;

  @override
  String toString() {
    return 'Message.tool(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_ToolImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_ToolImplCopyWith<_$Message_ToolImpl> get copyWith =>
      __$$Message_ToolImplCopyWithImpl<_$Message_ToolImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return tool(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return tool?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (tool != null) {
      return tool(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return tool(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return tool?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (tool != null) {
      return tool(this);
    }
    return orElse();
  }
}

abstract class Message_Tool extends Message {
  const factory Message_Tool({required final ToolMessage field0}) =
      _$Message_ToolImpl;
  const Message_Tool._() : super._();

  @override
  ToolMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_ToolImplCopyWith<_$Message_ToolImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$Message_DeveloperImplCopyWith<$Res> {
  factory _$$Message_DeveloperImplCopyWith(_$Message_DeveloperImpl value,
          $Res Function(_$Message_DeveloperImpl) then) =
      __$$Message_DeveloperImplCopyWithImpl<$Res>;
  @useResult
  $Res call({DeveloperMessage field0});
}

/// @nodoc
class __$$Message_DeveloperImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_DeveloperImpl>
    implements _$$Message_DeveloperImplCopyWith<$Res> {
  __$$Message_DeveloperImplCopyWithImpl(_$Message_DeveloperImpl _value,
      $Res Function(_$Message_DeveloperImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_DeveloperImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as DeveloperMessage,
    ));
  }
}

/// @nodoc

class _$Message_DeveloperImpl extends Message_Developer {
  const _$Message_DeveloperImpl({required this.field0}) : super._();

  @override
  final DeveloperMessage field0;

  @override
  String toString() {
    return 'Message.developer(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_DeveloperImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_DeveloperImplCopyWith<_$Message_DeveloperImpl> get copyWith =>
      __$$Message_DeveloperImplCopyWithImpl<_$Message_DeveloperImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return developer(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return developer?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (developer != null) {
      return developer(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return developer(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return developer?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (developer != null) {
      return developer(this);
    }
    return orElse();
  }
}

abstract class Message_Developer extends Message {
  const factory Message_Developer({required final DeveloperMessage field0}) =
      _$Message_DeveloperImpl;
  const Message_Developer._() : super._();

  @override
  DeveloperMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_DeveloperImplCopyWith<_$Message_DeveloperImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$Message_FunctionImplCopyWith<$Res> {
  factory _$$Message_FunctionImplCopyWith(_$Message_FunctionImpl value,
          $Res Function(_$Message_FunctionImpl) then) =
      __$$Message_FunctionImplCopyWithImpl<$Res>;
  @useResult
  $Res call({FunctionMessage field0});
}

/// @nodoc
class __$$Message_FunctionImplCopyWithImpl<$Res>
    extends _$MessageCopyWithImpl<$Res, _$Message_FunctionImpl>
    implements _$$Message_FunctionImplCopyWith<$Res> {
  __$$Message_FunctionImplCopyWithImpl(_$Message_FunctionImpl _value,
      $Res Function(_$Message_FunctionImpl) _then)
      : super(_value, _then);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$Message_FunctionImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as FunctionMessage,
    ));
  }
}

/// @nodoc

class _$Message_FunctionImpl extends Message_Function {
  const _$Message_FunctionImpl({required this.field0}) : super._();

  @override
  final FunctionMessage field0;

  @override
  String toString() {
    return 'Message.function(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$Message_FunctionImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$Message_FunctionImplCopyWith<_$Message_FunctionImpl> get copyWith =>
      __$$Message_FunctionImplCopyWithImpl<_$Message_FunctionImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(SystemMessage field0) system,
    required TResult Function(UserMessage field0) user,
    required TResult Function(AssistantMessage field0) assistant,
    required TResult Function(ToolMessage field0) tool,
    required TResult Function(DeveloperMessage field0) developer,
    required TResult Function(FunctionMessage field0) function,
  }) {
    return function(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(SystemMessage field0)? system,
    TResult? Function(UserMessage field0)? user,
    TResult? Function(AssistantMessage field0)? assistant,
    TResult? Function(ToolMessage field0)? tool,
    TResult? Function(DeveloperMessage field0)? developer,
    TResult? Function(FunctionMessage field0)? function,
  }) {
    return function?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(SystemMessage field0)? system,
    TResult Function(UserMessage field0)? user,
    TResult Function(AssistantMessage field0)? assistant,
    TResult Function(ToolMessage field0)? tool,
    TResult Function(DeveloperMessage field0)? developer,
    TResult Function(FunctionMessage field0)? function,
    required TResult orElse(),
  }) {
    if (function != null) {
      return function(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(Message_System value) system,
    required TResult Function(Message_User value) user,
    required TResult Function(Message_Assistant value) assistant,
    required TResult Function(Message_Tool value) tool,
    required TResult Function(Message_Developer value) developer,
    required TResult Function(Message_Function value) function,
  }) {
    return function(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(Message_System value)? system,
    TResult? Function(Message_User value)? user,
    TResult? Function(Message_Assistant value)? assistant,
    TResult? Function(Message_Tool value)? tool,
    TResult? Function(Message_Developer value)? developer,
    TResult? Function(Message_Function value)? function,
  }) {
    return function?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(Message_System value)? system,
    TResult Function(Message_User value)? user,
    TResult Function(Message_Assistant value)? assistant,
    TResult Function(Message_Tool value)? tool,
    TResult Function(Message_Developer value)? developer,
    TResult Function(Message_Function value)? function,
    required TResult orElse(),
  }) {
    if (function != null) {
      return function(this);
    }
    return orElse();
  }
}

abstract class Message_Function extends Message {
  const factory Message_Function({required final FunctionMessage field0}) =
      _$Message_FunctionImpl;
  const Message_Function._() : super._();

  @override
  FunctionMessage get field0;

  /// Create a copy of Message
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$Message_FunctionImplCopyWith<_$Message_FunctionImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$ModerationInput {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ModerationInput_Single value) single,
    required TResult Function(ModerationInput_Multiple value) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ModerationInput_Single value)? single,
    TResult? Function(ModerationInput_Multiple value)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ModerationInput_Single value)? single,
    TResult Function(ModerationInput_Multiple value)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ModerationInputCopyWith<$Res> {
  factory $ModerationInputCopyWith(
          ModerationInput value, $Res Function(ModerationInput) then) =
      _$ModerationInputCopyWithImpl<$Res, ModerationInput>;
}

/// @nodoc
class _$ModerationInputCopyWithImpl<$Res, $Val extends ModerationInput>
    implements $ModerationInputCopyWith<$Res> {
  _$ModerationInputCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$ModerationInput_SingleImplCopyWith<$Res> {
  factory _$$ModerationInput_SingleImplCopyWith(
          _$ModerationInput_SingleImpl value,
          $Res Function(_$ModerationInput_SingleImpl) then) =
      __$$ModerationInput_SingleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$ModerationInput_SingleImplCopyWithImpl<$Res>
    extends _$ModerationInputCopyWithImpl<$Res, _$ModerationInput_SingleImpl>
    implements _$$ModerationInput_SingleImplCopyWith<$Res> {
  __$$ModerationInput_SingleImplCopyWithImpl(
      _$ModerationInput_SingleImpl _value,
      $Res Function(_$ModerationInput_SingleImpl) _then)
      : super(_value, _then);

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ModerationInput_SingleImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$ModerationInput_SingleImpl extends ModerationInput_Single {
  const _$ModerationInput_SingleImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'ModerationInput.single(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ModerationInput_SingleImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ModerationInput_SingleImplCopyWith<_$ModerationInput_SingleImpl>
      get copyWith => __$$ModerationInput_SingleImplCopyWithImpl<
          _$ModerationInput_SingleImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return single(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return single?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ModerationInput_Single value) single,
    required TResult Function(ModerationInput_Multiple value) multiple,
  }) {
    return single(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ModerationInput_Single value)? single,
    TResult? Function(ModerationInput_Multiple value)? multiple,
  }) {
    return single?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ModerationInput_Single value)? single,
    TResult Function(ModerationInput_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(this);
    }
    return orElse();
  }
}

abstract class ModerationInput_Single extends ModerationInput {
  const factory ModerationInput_Single({required final String field0}) =
      _$ModerationInput_SingleImpl;
  const ModerationInput_Single._() : super._();

  @override
  String get field0;

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ModerationInput_SingleImplCopyWith<_$ModerationInput_SingleImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ModerationInput_MultipleImplCopyWith<$Res> {
  factory _$$ModerationInput_MultipleImplCopyWith(
          _$ModerationInput_MultipleImpl value,
          $Res Function(_$ModerationInput_MultipleImpl) then) =
      __$$ModerationInput_MultipleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<String> field0});
}

/// @nodoc
class __$$ModerationInput_MultipleImplCopyWithImpl<$Res>
    extends _$ModerationInputCopyWithImpl<$Res, _$ModerationInput_MultipleImpl>
    implements _$$ModerationInput_MultipleImplCopyWith<$Res> {
  __$$ModerationInput_MultipleImplCopyWithImpl(
      _$ModerationInput_MultipleImpl _value,
      $Res Function(_$ModerationInput_MultipleImpl) _then)
      : super(_value, _then);

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ModerationInput_MultipleImpl(
      field0: null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<String>,
    ));
  }
}

/// @nodoc

class _$ModerationInput_MultipleImpl extends ModerationInput_Multiple {
  const _$ModerationInput_MultipleImpl({required final List<String> field0})
      : _field0 = field0,
        super._();

  final List<String> _field0;
  @override
  List<String> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'ModerationInput.multiple(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ModerationInput_MultipleImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ModerationInput_MultipleImplCopyWith<_$ModerationInput_MultipleImpl>
      get copyWith => __$$ModerationInput_MultipleImplCopyWithImpl<
          _$ModerationInput_MultipleImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return multiple(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return multiple?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ModerationInput_Single value) single,
    required TResult Function(ModerationInput_Multiple value) multiple,
  }) {
    return multiple(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ModerationInput_Single value)? single,
    TResult? Function(ModerationInput_Multiple value)? multiple,
  }) {
    return multiple?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ModerationInput_Single value)? single,
    TResult Function(ModerationInput_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(this);
    }
    return orElse();
  }
}

abstract class ModerationInput_Multiple extends ModerationInput {
  const factory ModerationInput_Multiple({required final List<String> field0}) =
      _$ModerationInput_MultipleImpl;
  const ModerationInput_Multiple._() : super._();

  @override
  List<String> get field0;

  /// Create a copy of ModerationInput
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ModerationInput_MultipleImplCopyWith<_$ModerationInput_MultipleImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$OcrDocument {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String url) url,
    required TResult Function(String data, String mediaType) base64,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String url)? url,
    TResult? Function(String data, String mediaType)? base64,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String url)? url,
    TResult Function(String data, String mediaType)? base64,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OcrDocument_Url value) url,
    required TResult Function(OcrDocument_Base64 value) base64,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OcrDocument_Url value)? url,
    TResult? Function(OcrDocument_Base64 value)? base64,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OcrDocument_Url value)? url,
    TResult Function(OcrDocument_Base64 value)? base64,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $OcrDocumentCopyWith<$Res> {
  factory $OcrDocumentCopyWith(
          OcrDocument value, $Res Function(OcrDocument) then) =
      _$OcrDocumentCopyWithImpl<$Res, OcrDocument>;
}

/// @nodoc
class _$OcrDocumentCopyWithImpl<$Res, $Val extends OcrDocument>
    implements $OcrDocumentCopyWith<$Res> {
  _$OcrDocumentCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$OcrDocument_UrlImplCopyWith<$Res> {
  factory _$$OcrDocument_UrlImplCopyWith(_$OcrDocument_UrlImpl value,
          $Res Function(_$OcrDocument_UrlImpl) then) =
      __$$OcrDocument_UrlImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String url});
}

/// @nodoc
class __$$OcrDocument_UrlImplCopyWithImpl<$Res>
    extends _$OcrDocumentCopyWithImpl<$Res, _$OcrDocument_UrlImpl>
    implements _$$OcrDocument_UrlImplCopyWith<$Res> {
  __$$OcrDocument_UrlImplCopyWithImpl(
      _$OcrDocument_UrlImpl _value, $Res Function(_$OcrDocument_UrlImpl) _then)
      : super(_value, _then);

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? url = null,
  }) {
    return _then(_$OcrDocument_UrlImpl(
      url: null == url
          ? _value.url
          : url // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OcrDocument_UrlImpl extends OcrDocument_Url {
  const _$OcrDocument_UrlImpl({required this.url}) : super._();

  @override
  final String url;

  @override
  String toString() {
    return 'OcrDocument.url(url: $url)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OcrDocument_UrlImpl &&
            (identical(other.url, url) || other.url == url));
  }

  @override
  int get hashCode => Object.hash(runtimeType, url);

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$OcrDocument_UrlImplCopyWith<_$OcrDocument_UrlImpl> get copyWith =>
      __$$OcrDocument_UrlImplCopyWithImpl<_$OcrDocument_UrlImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String url) url,
    required TResult Function(String data, String mediaType) base64,
  }) {
    return url(this.url);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String url)? url,
    TResult? Function(String data, String mediaType)? base64,
  }) {
    return url?.call(this.url);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String url)? url,
    TResult Function(String data, String mediaType)? base64,
    required TResult orElse(),
  }) {
    if (url != null) {
      return url(this.url);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OcrDocument_Url value) url,
    required TResult Function(OcrDocument_Base64 value) base64,
  }) {
    return url(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OcrDocument_Url value)? url,
    TResult? Function(OcrDocument_Base64 value)? base64,
  }) {
    return url?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OcrDocument_Url value)? url,
    TResult Function(OcrDocument_Base64 value)? base64,
    required TResult orElse(),
  }) {
    if (url != null) {
      return url(this);
    }
    return orElse();
  }
}

abstract class OcrDocument_Url extends OcrDocument {
  const factory OcrDocument_Url({required final String url}) =
      _$OcrDocument_UrlImpl;
  const OcrDocument_Url._() : super._();

  String get url;

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$OcrDocument_UrlImplCopyWith<_$OcrDocument_UrlImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OcrDocument_Base64ImplCopyWith<$Res> {
  factory _$$OcrDocument_Base64ImplCopyWith(_$OcrDocument_Base64Impl value,
          $Res Function(_$OcrDocument_Base64Impl) then) =
      __$$OcrDocument_Base64ImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String data, String mediaType});
}

/// @nodoc
class __$$OcrDocument_Base64ImplCopyWithImpl<$Res>
    extends _$OcrDocumentCopyWithImpl<$Res, _$OcrDocument_Base64Impl>
    implements _$$OcrDocument_Base64ImplCopyWith<$Res> {
  __$$OcrDocument_Base64ImplCopyWithImpl(_$OcrDocument_Base64Impl _value,
      $Res Function(_$OcrDocument_Base64Impl) _then)
      : super(_value, _then);

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? data = null,
    Object? mediaType = null,
  }) {
    return _then(_$OcrDocument_Base64Impl(
      data: null == data
          ? _value.data
          : data // ignore: cast_nullable_to_non_nullable
              as String,
      mediaType: null == mediaType
          ? _value.mediaType
          : mediaType // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OcrDocument_Base64Impl extends OcrDocument_Base64 {
  const _$OcrDocument_Base64Impl({required this.data, required this.mediaType})
      : super._();

  @override
  final String data;
  @override
  final String mediaType;

  @override
  String toString() {
    return 'OcrDocument.base64(data: $data, mediaType: $mediaType)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OcrDocument_Base64Impl &&
            (identical(other.data, data) || other.data == data) &&
            (identical(other.mediaType, mediaType) ||
                other.mediaType == mediaType));
  }

  @override
  int get hashCode => Object.hash(runtimeType, data, mediaType);

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$OcrDocument_Base64ImplCopyWith<_$OcrDocument_Base64Impl> get copyWith =>
      __$$OcrDocument_Base64ImplCopyWithImpl<_$OcrDocument_Base64Impl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String url) url,
    required TResult Function(String data, String mediaType) base64,
  }) {
    return base64(data, mediaType);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String url)? url,
    TResult? Function(String data, String mediaType)? base64,
  }) {
    return base64?.call(data, mediaType);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String url)? url,
    TResult Function(String data, String mediaType)? base64,
    required TResult orElse(),
  }) {
    if (base64 != null) {
      return base64(data, mediaType);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OcrDocument_Url value) url,
    required TResult Function(OcrDocument_Base64 value) base64,
  }) {
    return base64(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OcrDocument_Url value)? url,
    TResult? Function(OcrDocument_Base64 value)? base64,
  }) {
    return base64?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OcrDocument_Url value)? url,
    TResult Function(OcrDocument_Base64 value)? base64,
    required TResult orElse(),
  }) {
    if (base64 != null) {
      return base64(this);
    }
    return orElse();
  }
}

abstract class OcrDocument_Base64 extends OcrDocument {
  const factory OcrDocument_Base64(
      {required final String data,
      required final String mediaType}) = _$OcrDocument_Base64Impl;
  const OcrDocument_Base64._() : super._();

  String get data;
  String get mediaType;

  /// Create a copy of OcrDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$OcrDocument_Base64ImplCopyWith<_$OcrDocument_Base64Impl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$RerankDocument {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(String text) object,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(String text)? object,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(String text)? object,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(RerankDocument_Text value) text,
    required TResult Function(RerankDocument_Object value) object,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(RerankDocument_Text value)? text,
    TResult? Function(RerankDocument_Object value)? object,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(RerankDocument_Text value)? text,
    TResult Function(RerankDocument_Object value)? object,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $RerankDocumentCopyWith<$Res> {
  factory $RerankDocumentCopyWith(
          RerankDocument value, $Res Function(RerankDocument) then) =
      _$RerankDocumentCopyWithImpl<$Res, RerankDocument>;
}

/// @nodoc
class _$RerankDocumentCopyWithImpl<$Res, $Val extends RerankDocument>
    implements $RerankDocumentCopyWith<$Res> {
  _$RerankDocumentCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$RerankDocument_TextImplCopyWith<$Res> {
  factory _$$RerankDocument_TextImplCopyWith(_$RerankDocument_TextImpl value,
          $Res Function(_$RerankDocument_TextImpl) then) =
      __$$RerankDocument_TextImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$RerankDocument_TextImplCopyWithImpl<$Res>
    extends _$RerankDocumentCopyWithImpl<$Res, _$RerankDocument_TextImpl>
    implements _$$RerankDocument_TextImplCopyWith<$Res> {
  __$$RerankDocument_TextImplCopyWithImpl(_$RerankDocument_TextImpl _value,
      $Res Function(_$RerankDocument_TextImpl) _then)
      : super(_value, _then);

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$RerankDocument_TextImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$RerankDocument_TextImpl extends RerankDocument_Text {
  const _$RerankDocument_TextImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'RerankDocument.text(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RerankDocument_TextImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RerankDocument_TextImplCopyWith<_$RerankDocument_TextImpl> get copyWith =>
      __$$RerankDocument_TextImplCopyWithImpl<_$RerankDocument_TextImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(String text) object,
  }) {
    return text(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(String text)? object,
  }) {
    return text?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(String text)? object,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(RerankDocument_Text value) text,
    required TResult Function(RerankDocument_Object value) object,
  }) {
    return text(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(RerankDocument_Text value)? text,
    TResult? Function(RerankDocument_Object value)? object,
  }) {
    return text?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(RerankDocument_Text value)? text,
    TResult Function(RerankDocument_Object value)? object,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this);
    }
    return orElse();
  }
}

abstract class RerankDocument_Text extends RerankDocument {
  const factory RerankDocument_Text({required final String field0}) =
      _$RerankDocument_TextImpl;
  const RerankDocument_Text._() : super._();

  String get field0;

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RerankDocument_TextImplCopyWith<_$RerankDocument_TextImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$RerankDocument_ObjectImplCopyWith<$Res> {
  factory _$$RerankDocument_ObjectImplCopyWith(
          _$RerankDocument_ObjectImpl value,
          $Res Function(_$RerankDocument_ObjectImpl) then) =
      __$$RerankDocument_ObjectImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String text});
}

/// @nodoc
class __$$RerankDocument_ObjectImplCopyWithImpl<$Res>
    extends _$RerankDocumentCopyWithImpl<$Res, _$RerankDocument_ObjectImpl>
    implements _$$RerankDocument_ObjectImplCopyWith<$Res> {
  __$$RerankDocument_ObjectImplCopyWithImpl(_$RerankDocument_ObjectImpl _value,
      $Res Function(_$RerankDocument_ObjectImpl) _then)
      : super(_value, _then);

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? text = null,
  }) {
    return _then(_$RerankDocument_ObjectImpl(
      text: null == text
          ? _value.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$RerankDocument_ObjectImpl extends RerankDocument_Object {
  const _$RerankDocument_ObjectImpl({required this.text}) : super._();

  @override
  final String text;

  @override
  String toString() {
    return 'RerankDocument.object(text: $text)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$RerankDocument_ObjectImpl &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$RerankDocument_ObjectImplCopyWith<_$RerankDocument_ObjectImpl>
      get copyWith => __$$RerankDocument_ObjectImplCopyWithImpl<
          _$RerankDocument_ObjectImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(String text) object,
  }) {
    return object(this.text);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(String text)? object,
  }) {
    return object?.call(this.text);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(String text)? object,
    required TResult orElse(),
  }) {
    if (object != null) {
      return object(this.text);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(RerankDocument_Text value) text,
    required TResult Function(RerankDocument_Object value) object,
  }) {
    return object(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(RerankDocument_Text value)? text,
    TResult? Function(RerankDocument_Object value)? object,
  }) {
    return object?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(RerankDocument_Text value)? text,
    TResult Function(RerankDocument_Object value)? object,
    required TResult orElse(),
  }) {
    if (object != null) {
      return object(this);
    }
    return orElse();
  }
}

abstract class RerankDocument_Object extends RerankDocument {
  const factory RerankDocument_Object({required final String text}) =
      _$RerankDocument_ObjectImpl;
  const RerankDocument_Object._() : super._();

  String get text;

  /// Create a copy of RerankDocument
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$RerankDocument_ObjectImplCopyWith<_$RerankDocument_ObjectImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$ResponseFormat {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() text,
    required TResult Function() jsonObject,
    required TResult Function(JsonSchemaFormat jsonSchema) jsonSchema,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? text,
    TResult? Function()? jsonObject,
    TResult? Function(JsonSchemaFormat jsonSchema)? jsonSchema,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? text,
    TResult Function()? jsonObject,
    TResult Function(JsonSchemaFormat jsonSchema)? jsonSchema,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ResponseFormat_Text value) text,
    required TResult Function(ResponseFormat_JsonObject value) jsonObject,
    required TResult Function(ResponseFormat_JsonSchema value) jsonSchema,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ResponseFormat_Text value)? text,
    TResult? Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult? Function(ResponseFormat_JsonSchema value)? jsonSchema,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ResponseFormat_Text value)? text,
    TResult Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult Function(ResponseFormat_JsonSchema value)? jsonSchema,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ResponseFormatCopyWith<$Res> {
  factory $ResponseFormatCopyWith(
          ResponseFormat value, $Res Function(ResponseFormat) then) =
      _$ResponseFormatCopyWithImpl<$Res, ResponseFormat>;
}

/// @nodoc
class _$ResponseFormatCopyWithImpl<$Res, $Val extends ResponseFormat>
    implements $ResponseFormatCopyWith<$Res> {
  _$ResponseFormatCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$ResponseFormat_TextImplCopyWith<$Res> {
  factory _$$ResponseFormat_TextImplCopyWith(_$ResponseFormat_TextImpl value,
          $Res Function(_$ResponseFormat_TextImpl) then) =
      __$$ResponseFormat_TextImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$ResponseFormat_TextImplCopyWithImpl<$Res>
    extends _$ResponseFormatCopyWithImpl<$Res, _$ResponseFormat_TextImpl>
    implements _$$ResponseFormat_TextImplCopyWith<$Res> {
  __$$ResponseFormat_TextImplCopyWithImpl(_$ResponseFormat_TextImpl _value,
      $Res Function(_$ResponseFormat_TextImpl) _then)
      : super(_value, _then);

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$ResponseFormat_TextImpl extends ResponseFormat_Text {
  const _$ResponseFormat_TextImpl() : super._();

  @override
  String toString() {
    return 'ResponseFormat.text()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ResponseFormat_TextImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() text,
    required TResult Function() jsonObject,
    required TResult Function(JsonSchemaFormat jsonSchema) jsonSchema,
  }) {
    return text();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? text,
    TResult? Function()? jsonObject,
    TResult? Function(JsonSchemaFormat jsonSchema)? jsonSchema,
  }) {
    return text?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? text,
    TResult Function()? jsonObject,
    TResult Function(JsonSchemaFormat jsonSchema)? jsonSchema,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ResponseFormat_Text value) text,
    required TResult Function(ResponseFormat_JsonObject value) jsonObject,
    required TResult Function(ResponseFormat_JsonSchema value) jsonSchema,
  }) {
    return text(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ResponseFormat_Text value)? text,
    TResult? Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult? Function(ResponseFormat_JsonSchema value)? jsonSchema,
  }) {
    return text?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ResponseFormat_Text value)? text,
    TResult Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult Function(ResponseFormat_JsonSchema value)? jsonSchema,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this);
    }
    return orElse();
  }
}

abstract class ResponseFormat_Text extends ResponseFormat {
  const factory ResponseFormat_Text() = _$ResponseFormat_TextImpl;
  const ResponseFormat_Text._() : super._();
}

/// @nodoc
abstract class _$$ResponseFormat_JsonObjectImplCopyWith<$Res> {
  factory _$$ResponseFormat_JsonObjectImplCopyWith(
          _$ResponseFormat_JsonObjectImpl value,
          $Res Function(_$ResponseFormat_JsonObjectImpl) then) =
      __$$ResponseFormat_JsonObjectImplCopyWithImpl<$Res>;
}

/// @nodoc
class __$$ResponseFormat_JsonObjectImplCopyWithImpl<$Res>
    extends _$ResponseFormatCopyWithImpl<$Res, _$ResponseFormat_JsonObjectImpl>
    implements _$$ResponseFormat_JsonObjectImplCopyWith<$Res> {
  __$$ResponseFormat_JsonObjectImplCopyWithImpl(
      _$ResponseFormat_JsonObjectImpl _value,
      $Res Function(_$ResponseFormat_JsonObjectImpl) _then)
      : super(_value, _then);

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc

class _$ResponseFormat_JsonObjectImpl extends ResponseFormat_JsonObject {
  const _$ResponseFormat_JsonObjectImpl() : super._();

  @override
  String toString() {
    return 'ResponseFormat.jsonObject()';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ResponseFormat_JsonObjectImpl);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() text,
    required TResult Function() jsonObject,
    required TResult Function(JsonSchemaFormat jsonSchema) jsonSchema,
  }) {
    return jsonObject();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? text,
    TResult? Function()? jsonObject,
    TResult? Function(JsonSchemaFormat jsonSchema)? jsonSchema,
  }) {
    return jsonObject?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? text,
    TResult Function()? jsonObject,
    TResult Function(JsonSchemaFormat jsonSchema)? jsonSchema,
    required TResult orElse(),
  }) {
    if (jsonObject != null) {
      return jsonObject();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ResponseFormat_Text value) text,
    required TResult Function(ResponseFormat_JsonObject value) jsonObject,
    required TResult Function(ResponseFormat_JsonSchema value) jsonSchema,
  }) {
    return jsonObject(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ResponseFormat_Text value)? text,
    TResult? Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult? Function(ResponseFormat_JsonSchema value)? jsonSchema,
  }) {
    return jsonObject?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ResponseFormat_Text value)? text,
    TResult Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult Function(ResponseFormat_JsonSchema value)? jsonSchema,
    required TResult orElse(),
  }) {
    if (jsonObject != null) {
      return jsonObject(this);
    }
    return orElse();
  }
}

abstract class ResponseFormat_JsonObject extends ResponseFormat {
  const factory ResponseFormat_JsonObject() = _$ResponseFormat_JsonObjectImpl;
  const ResponseFormat_JsonObject._() : super._();
}

/// @nodoc
abstract class _$$ResponseFormat_JsonSchemaImplCopyWith<$Res> {
  factory _$$ResponseFormat_JsonSchemaImplCopyWith(
          _$ResponseFormat_JsonSchemaImpl value,
          $Res Function(_$ResponseFormat_JsonSchemaImpl) then) =
      __$$ResponseFormat_JsonSchemaImplCopyWithImpl<$Res>;
  @useResult
  $Res call({JsonSchemaFormat jsonSchema});
}

/// @nodoc
class __$$ResponseFormat_JsonSchemaImplCopyWithImpl<$Res>
    extends _$ResponseFormatCopyWithImpl<$Res, _$ResponseFormat_JsonSchemaImpl>
    implements _$$ResponseFormat_JsonSchemaImplCopyWith<$Res> {
  __$$ResponseFormat_JsonSchemaImplCopyWithImpl(
      _$ResponseFormat_JsonSchemaImpl _value,
      $Res Function(_$ResponseFormat_JsonSchemaImpl) _then)
      : super(_value, _then);

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? jsonSchema = null,
  }) {
    return _then(_$ResponseFormat_JsonSchemaImpl(
      jsonSchema: null == jsonSchema
          ? _value.jsonSchema
          : jsonSchema // ignore: cast_nullable_to_non_nullable
              as JsonSchemaFormat,
    ));
  }
}

/// @nodoc

class _$ResponseFormat_JsonSchemaImpl extends ResponseFormat_JsonSchema {
  const _$ResponseFormat_JsonSchemaImpl({required this.jsonSchema}) : super._();

  @override
  final JsonSchemaFormat jsonSchema;

  @override
  String toString() {
    return 'ResponseFormat.jsonSchema(jsonSchema: $jsonSchema)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ResponseFormat_JsonSchemaImpl &&
            (identical(other.jsonSchema, jsonSchema) ||
                other.jsonSchema == jsonSchema));
  }

  @override
  int get hashCode => Object.hash(runtimeType, jsonSchema);

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ResponseFormat_JsonSchemaImplCopyWith<_$ResponseFormat_JsonSchemaImpl>
      get copyWith => __$$ResponseFormat_JsonSchemaImplCopyWithImpl<
          _$ResponseFormat_JsonSchemaImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() text,
    required TResult Function() jsonObject,
    required TResult Function(JsonSchemaFormat jsonSchema) jsonSchema,
  }) {
    return jsonSchema(this.jsonSchema);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? text,
    TResult? Function()? jsonObject,
    TResult? Function(JsonSchemaFormat jsonSchema)? jsonSchema,
  }) {
    return jsonSchema?.call(this.jsonSchema);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? text,
    TResult Function()? jsonObject,
    TResult Function(JsonSchemaFormat jsonSchema)? jsonSchema,
    required TResult orElse(),
  }) {
    if (jsonSchema != null) {
      return jsonSchema(this.jsonSchema);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ResponseFormat_Text value) text,
    required TResult Function(ResponseFormat_JsonObject value) jsonObject,
    required TResult Function(ResponseFormat_JsonSchema value) jsonSchema,
  }) {
    return jsonSchema(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ResponseFormat_Text value)? text,
    TResult? Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult? Function(ResponseFormat_JsonSchema value)? jsonSchema,
  }) {
    return jsonSchema?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ResponseFormat_Text value)? text,
    TResult Function(ResponseFormat_JsonObject value)? jsonObject,
    TResult Function(ResponseFormat_JsonSchema value)? jsonSchema,
    required TResult orElse(),
  }) {
    if (jsonSchema != null) {
      return jsonSchema(this);
    }
    return orElse();
  }
}

abstract class ResponseFormat_JsonSchema extends ResponseFormat {
  const factory ResponseFormat_JsonSchema(
          {required final JsonSchemaFormat jsonSchema}) =
      _$ResponseFormat_JsonSchemaImpl;
  const ResponseFormat_JsonSchema._() : super._();

  JsonSchemaFormat get jsonSchema;

  /// Create a copy of ResponseFormat
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ResponseFormat_JsonSchemaImplCopyWith<_$ResponseFormat_JsonSchemaImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$StopSequence {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StopSequence_Single value) single,
    required TResult Function(StopSequence_Multiple value) multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StopSequence_Single value)? single,
    TResult? Function(StopSequence_Multiple value)? multiple,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StopSequence_Single value)? single,
    TResult Function(StopSequence_Multiple value)? multiple,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $StopSequenceCopyWith<$Res> {
  factory $StopSequenceCopyWith(
          StopSequence value, $Res Function(StopSequence) then) =
      _$StopSequenceCopyWithImpl<$Res, StopSequence>;
}

/// @nodoc
class _$StopSequenceCopyWithImpl<$Res, $Val extends StopSequence>
    implements $StopSequenceCopyWith<$Res> {
  _$StopSequenceCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$StopSequence_SingleImplCopyWith<$Res> {
  factory _$$StopSequence_SingleImplCopyWith(_$StopSequence_SingleImpl value,
          $Res Function(_$StopSequence_SingleImpl) then) =
      __$$StopSequence_SingleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$StopSequence_SingleImplCopyWithImpl<$Res>
    extends _$StopSequenceCopyWithImpl<$Res, _$StopSequence_SingleImpl>
    implements _$$StopSequence_SingleImplCopyWith<$Res> {
  __$$StopSequence_SingleImplCopyWithImpl(_$StopSequence_SingleImpl _value,
      $Res Function(_$StopSequence_SingleImpl) _then)
      : super(_value, _then);

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$StopSequence_SingleImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$StopSequence_SingleImpl extends StopSequence_Single {
  const _$StopSequence_SingleImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'StopSequence.single(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$StopSequence_SingleImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$StopSequence_SingleImplCopyWith<_$StopSequence_SingleImpl> get copyWith =>
      __$$StopSequence_SingleImplCopyWithImpl<_$StopSequence_SingleImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return single(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return single?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StopSequence_Single value) single,
    required TResult Function(StopSequence_Multiple value) multiple,
  }) {
    return single(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StopSequence_Single value)? single,
    TResult? Function(StopSequence_Multiple value)? multiple,
  }) {
    return single?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StopSequence_Single value)? single,
    TResult Function(StopSequence_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (single != null) {
      return single(this);
    }
    return orElse();
  }
}

abstract class StopSequence_Single extends StopSequence {
  const factory StopSequence_Single({required final String field0}) =
      _$StopSequence_SingleImpl;
  const StopSequence_Single._() : super._();

  @override
  String get field0;

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$StopSequence_SingleImplCopyWith<_$StopSequence_SingleImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$StopSequence_MultipleImplCopyWith<$Res> {
  factory _$$StopSequence_MultipleImplCopyWith(
          _$StopSequence_MultipleImpl value,
          $Res Function(_$StopSequence_MultipleImpl) then) =
      __$$StopSequence_MultipleImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<String> field0});
}

/// @nodoc
class __$$StopSequence_MultipleImplCopyWithImpl<$Res>
    extends _$StopSequenceCopyWithImpl<$Res, _$StopSequence_MultipleImpl>
    implements _$$StopSequence_MultipleImplCopyWith<$Res> {
  __$$StopSequence_MultipleImplCopyWithImpl(_$StopSequence_MultipleImpl _value,
      $Res Function(_$StopSequence_MultipleImpl) _then)
      : super(_value, _then);

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$StopSequence_MultipleImpl(
      field0: null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<String>,
    ));
  }
}

/// @nodoc

class _$StopSequence_MultipleImpl extends StopSequence_Multiple {
  const _$StopSequence_MultipleImpl({required final List<String> field0})
      : _field0 = field0,
        super._();

  final List<String> _field0;
  @override
  List<String> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'StopSequence.multiple(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$StopSequence_MultipleImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$StopSequence_MultipleImplCopyWith<_$StopSequence_MultipleImpl>
      get copyWith => __$$StopSequence_MultipleImplCopyWithImpl<
          _$StopSequence_MultipleImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) single,
    required TResult Function(List<String> field0) multiple,
  }) {
    return multiple(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? single,
    TResult? Function(List<String> field0)? multiple,
  }) {
    return multiple?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? single,
    TResult Function(List<String> field0)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(StopSequence_Single value) single,
    required TResult Function(StopSequence_Multiple value) multiple,
  }) {
    return multiple(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(StopSequence_Single value)? single,
    TResult? Function(StopSequence_Multiple value)? multiple,
  }) {
    return multiple?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(StopSequence_Single value)? single,
    TResult Function(StopSequence_Multiple value)? multiple,
    required TResult orElse(),
  }) {
    if (multiple != null) {
      return multiple(this);
    }
    return orElse();
  }
}

abstract class StopSequence_Multiple extends StopSequence {
  const factory StopSequence_Multiple({required final List<String> field0}) =
      _$StopSequence_MultipleImpl;
  const StopSequence_Multiple._() : super._();

  @override
  List<String> get field0;

  /// Create a copy of StopSequence
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$StopSequence_MultipleImplCopyWith<_$StopSequence_MultipleImpl>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$ToolChoice {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ToolChoiceMode field0) mode,
    required TResult Function(SpecificToolChoice field0) specific,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ToolChoiceMode field0)? mode,
    TResult? Function(SpecificToolChoice field0)? specific,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ToolChoiceMode field0)? mode,
    TResult Function(SpecificToolChoice field0)? specific,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ToolChoice_Mode value) mode,
    required TResult Function(ToolChoice_Specific value) specific,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ToolChoice_Mode value)? mode,
    TResult? Function(ToolChoice_Specific value)? specific,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ToolChoice_Mode value)? mode,
    TResult Function(ToolChoice_Specific value)? specific,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ToolChoiceCopyWith<$Res> {
  factory $ToolChoiceCopyWith(
          ToolChoice value, $Res Function(ToolChoice) then) =
      _$ToolChoiceCopyWithImpl<$Res, ToolChoice>;
}

/// @nodoc
class _$ToolChoiceCopyWithImpl<$Res, $Val extends ToolChoice>
    implements $ToolChoiceCopyWith<$Res> {
  _$ToolChoiceCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$ToolChoice_ModeImplCopyWith<$Res> {
  factory _$$ToolChoice_ModeImplCopyWith(_$ToolChoice_ModeImpl value,
          $Res Function(_$ToolChoice_ModeImpl) then) =
      __$$ToolChoice_ModeImplCopyWithImpl<$Res>;
  @useResult
  $Res call({ToolChoiceMode field0});
}

/// @nodoc
class __$$ToolChoice_ModeImplCopyWithImpl<$Res>
    extends _$ToolChoiceCopyWithImpl<$Res, _$ToolChoice_ModeImpl>
    implements _$$ToolChoice_ModeImplCopyWith<$Res> {
  __$$ToolChoice_ModeImplCopyWithImpl(
      _$ToolChoice_ModeImpl _value, $Res Function(_$ToolChoice_ModeImpl) _then)
      : super(_value, _then);

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ToolChoice_ModeImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ToolChoiceMode,
    ));
  }
}

/// @nodoc

class _$ToolChoice_ModeImpl extends ToolChoice_Mode {
  const _$ToolChoice_ModeImpl({required this.field0}) : super._();

  @override
  final ToolChoiceMode field0;

  @override
  String toString() {
    return 'ToolChoice.mode(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ToolChoice_ModeImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ToolChoice_ModeImplCopyWith<_$ToolChoice_ModeImpl> get copyWith =>
      __$$ToolChoice_ModeImplCopyWithImpl<_$ToolChoice_ModeImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ToolChoiceMode field0) mode,
    required TResult Function(SpecificToolChoice field0) specific,
  }) {
    return mode(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ToolChoiceMode field0)? mode,
    TResult? Function(SpecificToolChoice field0)? specific,
  }) {
    return mode?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ToolChoiceMode field0)? mode,
    TResult Function(SpecificToolChoice field0)? specific,
    required TResult orElse(),
  }) {
    if (mode != null) {
      return mode(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ToolChoice_Mode value) mode,
    required TResult Function(ToolChoice_Specific value) specific,
  }) {
    return mode(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ToolChoice_Mode value)? mode,
    TResult? Function(ToolChoice_Specific value)? specific,
  }) {
    return mode?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ToolChoice_Mode value)? mode,
    TResult Function(ToolChoice_Specific value)? specific,
    required TResult orElse(),
  }) {
    if (mode != null) {
      return mode(this);
    }
    return orElse();
  }
}

abstract class ToolChoice_Mode extends ToolChoice {
  const factory ToolChoice_Mode({required final ToolChoiceMode field0}) =
      _$ToolChoice_ModeImpl;
  const ToolChoice_Mode._() : super._();

  @override
  ToolChoiceMode get field0;

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ToolChoice_ModeImplCopyWith<_$ToolChoice_ModeImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ToolChoice_SpecificImplCopyWith<$Res> {
  factory _$$ToolChoice_SpecificImplCopyWith(_$ToolChoice_SpecificImpl value,
          $Res Function(_$ToolChoice_SpecificImpl) then) =
      __$$ToolChoice_SpecificImplCopyWithImpl<$Res>;
  @useResult
  $Res call({SpecificToolChoice field0});
}

/// @nodoc
class __$$ToolChoice_SpecificImplCopyWithImpl<$Res>
    extends _$ToolChoiceCopyWithImpl<$Res, _$ToolChoice_SpecificImpl>
    implements _$$ToolChoice_SpecificImplCopyWith<$Res> {
  __$$ToolChoice_SpecificImplCopyWithImpl(_$ToolChoice_SpecificImpl _value,
      $Res Function(_$ToolChoice_SpecificImpl) _then)
      : super(_value, _then);

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ToolChoice_SpecificImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as SpecificToolChoice,
    ));
  }
}

/// @nodoc

class _$ToolChoice_SpecificImpl extends ToolChoice_Specific {
  const _$ToolChoice_SpecificImpl({required this.field0}) : super._();

  @override
  final SpecificToolChoice field0;

  @override
  String toString() {
    return 'ToolChoice.specific(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ToolChoice_SpecificImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$ToolChoice_SpecificImplCopyWith<_$ToolChoice_SpecificImpl> get copyWith =>
      __$$ToolChoice_SpecificImplCopyWithImpl<_$ToolChoice_SpecificImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(ToolChoiceMode field0) mode,
    required TResult Function(SpecificToolChoice field0) specific,
  }) {
    return specific(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(ToolChoiceMode field0)? mode,
    TResult? Function(SpecificToolChoice field0)? specific,
  }) {
    return specific?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(ToolChoiceMode field0)? mode,
    TResult Function(SpecificToolChoice field0)? specific,
    required TResult orElse(),
  }) {
    if (specific != null) {
      return specific(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ToolChoice_Mode value) mode,
    required TResult Function(ToolChoice_Specific value) specific,
  }) {
    return specific(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ToolChoice_Mode value)? mode,
    TResult? Function(ToolChoice_Specific value)? specific,
  }) {
    return specific?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ToolChoice_Mode value)? mode,
    TResult Function(ToolChoice_Specific value)? specific,
    required TResult orElse(),
  }) {
    if (specific != null) {
      return specific(this);
    }
    return orElse();
  }
}

abstract class ToolChoice_Specific extends ToolChoice {
  const factory ToolChoice_Specific(
      {required final SpecificToolChoice field0}) = _$ToolChoice_SpecificImpl;
  const ToolChoice_Specific._() : super._();

  @override
  SpecificToolChoice get field0;

  /// Create a copy of ToolChoice
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$ToolChoice_SpecificImplCopyWith<_$ToolChoice_SpecificImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$UserContent {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(List<ContentPart> field0) parts,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(List<ContentPart> field0)? parts,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(List<ContentPart> field0)? parts,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UserContent_Text value) text,
    required TResult Function(UserContent_Parts value) parts,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UserContent_Text value)? text,
    TResult? Function(UserContent_Parts value)? parts,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UserContent_Text value)? text,
    TResult Function(UserContent_Parts value)? parts,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $UserContentCopyWith<$Res> {
  factory $UserContentCopyWith(
          UserContent value, $Res Function(UserContent) then) =
      _$UserContentCopyWithImpl<$Res, UserContent>;
}

/// @nodoc
class _$UserContentCopyWithImpl<$Res, $Val extends UserContent>
    implements $UserContentCopyWith<$Res> {
  _$UserContentCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$UserContent_TextImplCopyWith<$Res> {
  factory _$$UserContent_TextImplCopyWith(_$UserContent_TextImpl value,
          $Res Function(_$UserContent_TextImpl) then) =
      __$$UserContent_TextImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$UserContent_TextImplCopyWithImpl<$Res>
    extends _$UserContentCopyWithImpl<$Res, _$UserContent_TextImpl>
    implements _$$UserContent_TextImplCopyWith<$Res> {
  __$$UserContent_TextImplCopyWithImpl(_$UserContent_TextImpl _value,
      $Res Function(_$UserContent_TextImpl) _then)
      : super(_value, _then);

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UserContent_TextImpl(
      field0: null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$UserContent_TextImpl extends UserContent_Text {
  const _$UserContent_TextImpl({required this.field0}) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'UserContent.text(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UserContent_TextImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UserContent_TextImplCopyWith<_$UserContent_TextImpl> get copyWith =>
      __$$UserContent_TextImplCopyWithImpl<_$UserContent_TextImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(List<ContentPart> field0) parts,
  }) {
    return text(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(List<ContentPart> field0)? parts,
  }) {
    return text?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(List<ContentPart> field0)? parts,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UserContent_Text value) text,
    required TResult Function(UserContent_Parts value) parts,
  }) {
    return text(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UserContent_Text value)? text,
    TResult? Function(UserContent_Parts value)? parts,
  }) {
    return text?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UserContent_Text value)? text,
    TResult Function(UserContent_Parts value)? parts,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this);
    }
    return orElse();
  }
}

abstract class UserContent_Text extends UserContent {
  const factory UserContent_Text({required final String field0}) =
      _$UserContent_TextImpl;
  const UserContent_Text._() : super._();

  @override
  String get field0;

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UserContent_TextImplCopyWith<_$UserContent_TextImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$UserContent_PartsImplCopyWith<$Res> {
  factory _$$UserContent_PartsImplCopyWith(_$UserContent_PartsImpl value,
          $Res Function(_$UserContent_PartsImpl) then) =
      __$$UserContent_PartsImplCopyWithImpl<$Res>;
  @useResult
  $Res call({List<ContentPart> field0});
}

/// @nodoc
class __$$UserContent_PartsImplCopyWithImpl<$Res>
    extends _$UserContentCopyWithImpl<$Res, _$UserContent_PartsImpl>
    implements _$$UserContent_PartsImplCopyWith<$Res> {
  __$$UserContent_PartsImplCopyWithImpl(_$UserContent_PartsImpl _value,
      $Res Function(_$UserContent_PartsImpl) _then)
      : super(_value, _then);

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$UserContent_PartsImpl(
      field0: null == field0
          ? _value._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<ContentPart>,
    ));
  }
}

/// @nodoc

class _$UserContent_PartsImpl extends UserContent_Parts {
  const _$UserContent_PartsImpl({required final List<ContentPart> field0})
      : _field0 = field0,
        super._();

  final List<ContentPart> _field0;
  @override
  List<ContentPart> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  @override
  String toString() {
    return 'UserContent.parts(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$UserContent_PartsImpl &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$UserContent_PartsImplCopyWith<_$UserContent_PartsImpl> get copyWith =>
      __$$UserContent_PartsImplCopyWithImpl<_$UserContent_PartsImpl>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) text,
    required TResult Function(List<ContentPart> field0) parts,
  }) {
    return parts(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? text,
    TResult? Function(List<ContentPart> field0)? parts,
  }) {
    return parts?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? text,
    TResult Function(List<ContentPart> field0)? parts,
    required TResult orElse(),
  }) {
    if (parts != null) {
      return parts(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(UserContent_Text value) text,
    required TResult Function(UserContent_Parts value) parts,
  }) {
    return parts(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(UserContent_Text value)? text,
    TResult? Function(UserContent_Parts value)? parts,
  }) {
    return parts?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(UserContent_Text value)? text,
    TResult Function(UserContent_Parts value)? parts,
    required TResult orElse(),
  }) {
    if (parts != null) {
      return parts(this);
    }
    return orElse();
  }
}

abstract class UserContent_Parts extends UserContent {
  const factory UserContent_Parts({required final List<ContentPart> field0}) =
      _$UserContent_PartsImpl;
  const UserContent_Parts._() : super._();

  @override
  List<ContentPart> get field0;

  /// Create a copy of UserContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$UserContent_PartsImplCopyWith<_$UserContent_PartsImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
