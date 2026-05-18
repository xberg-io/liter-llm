// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'lib.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AuthHeaderFormat {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AuthHeaderFormat);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AuthHeaderFormat()';
}


}

/// @nodoc
class $AuthHeaderFormatCopyWith<$Res>  {
$AuthHeaderFormatCopyWith(AuthHeaderFormat _, $Res Function(AuthHeaderFormat) __);
}


/// Adds pattern-matching-related methods to [AuthHeaderFormat].
extension AuthHeaderFormatPatterns on AuthHeaderFormat {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AuthHeaderFormat_Bearer value)?  bearer,TResult Function( AuthHeaderFormat_ApiKey value)?  apiKey,TResult Function( AuthHeaderFormat_None value)?  none,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer() when bearer != null:
return bearer(_that);case AuthHeaderFormat_ApiKey() when apiKey != null:
return apiKey(_that);case AuthHeaderFormat_None() when none != null:
return none(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AuthHeaderFormat_Bearer value)  bearer,required TResult Function( AuthHeaderFormat_ApiKey value)  apiKey,required TResult Function( AuthHeaderFormat_None value)  none,}){
final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer():
return bearer(_that);case AuthHeaderFormat_ApiKey():
return apiKey(_that);case AuthHeaderFormat_None():
return none(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AuthHeaderFormat_Bearer value)?  bearer,TResult? Function( AuthHeaderFormat_ApiKey value)?  apiKey,TResult? Function( AuthHeaderFormat_None value)?  none,}){
final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer() when bearer != null:
return bearer(_that);case AuthHeaderFormat_ApiKey() when apiKey != null:
return apiKey(_that);case AuthHeaderFormat_None() when none != null:
return none(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  bearer,TResult Function( String field0)?  apiKey,TResult Function()?  none,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer() when bearer != null:
return bearer();case AuthHeaderFormat_ApiKey() when apiKey != null:
return apiKey(_that.field0);case AuthHeaderFormat_None() when none != null:
return none();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  bearer,required TResult Function( String field0)  apiKey,required TResult Function()  none,}) {final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer():
return bearer();case AuthHeaderFormat_ApiKey():
return apiKey(_that.field0);case AuthHeaderFormat_None():
return none();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  bearer,TResult? Function( String field0)?  apiKey,TResult? Function()?  none,}) {final _that = this;
switch (_that) {
case AuthHeaderFormat_Bearer() when bearer != null:
return bearer();case AuthHeaderFormat_ApiKey() when apiKey != null:
return apiKey(_that.field0);case AuthHeaderFormat_None() when none != null:
return none();case _:
  return null;

}
}

}

/// @nodoc


class AuthHeaderFormat_Bearer extends AuthHeaderFormat {
  const AuthHeaderFormat_Bearer(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AuthHeaderFormat_Bearer);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AuthHeaderFormat.bearer()';
}


}




/// @nodoc


class AuthHeaderFormat_ApiKey extends AuthHeaderFormat {
  const AuthHeaderFormat_ApiKey({required this.field0}): super._();


 final  String field0;

/// Create a copy of AuthHeaderFormat
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AuthHeaderFormat_ApiKeyCopyWith<AuthHeaderFormat_ApiKey> get copyWith => _$AuthHeaderFormat_ApiKeyCopyWithImpl<AuthHeaderFormat_ApiKey>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AuthHeaderFormat_ApiKey&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AuthHeaderFormat.apiKey(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AuthHeaderFormat_ApiKeyCopyWith<$Res> implements $AuthHeaderFormatCopyWith<$Res> {
  factory $AuthHeaderFormat_ApiKeyCopyWith(AuthHeaderFormat_ApiKey value, $Res Function(AuthHeaderFormat_ApiKey) _then) = _$AuthHeaderFormat_ApiKeyCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$AuthHeaderFormat_ApiKeyCopyWithImpl<$Res>
    implements $AuthHeaderFormat_ApiKeyCopyWith<$Res> {
  _$AuthHeaderFormat_ApiKeyCopyWithImpl(this._self, this._then);

  final AuthHeaderFormat_ApiKey _self;
  final $Res Function(AuthHeaderFormat_ApiKey) _then;

/// Create a copy of AuthHeaderFormat
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AuthHeaderFormat_ApiKey(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class AuthHeaderFormat_None extends AuthHeaderFormat {
  const AuthHeaderFormat_None(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AuthHeaderFormat_None);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AuthHeaderFormat.none()';
}


}




/// @nodoc
mixin _$CacheBackend {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CacheBackend);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CacheBackend()';
}


}

/// @nodoc
class $CacheBackendCopyWith<$Res>  {
$CacheBackendCopyWith(CacheBackend _, $Res Function(CacheBackend) __);
}


/// Adds pattern-matching-related methods to [CacheBackend].
extension CacheBackendPatterns on CacheBackend {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( CacheBackend_Memory value)?  memory,TResult Function( CacheBackend_OpenDal value)?  openDal,required TResult orElse(),}){
final _that = this;
switch (_that) {
case CacheBackend_Memory() when memory != null:
return memory(_that);case CacheBackend_OpenDal() when openDal != null:
return openDal(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( CacheBackend_Memory value)  memory,required TResult Function( CacheBackend_OpenDal value)  openDal,}){
final _that = this;
switch (_that) {
case CacheBackend_Memory():
return memory(_that);case CacheBackend_OpenDal():
return openDal(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( CacheBackend_Memory value)?  memory,TResult? Function( CacheBackend_OpenDal value)?  openDal,}){
final _that = this;
switch (_that) {
case CacheBackend_Memory() when memory != null:
return memory(_that);case CacheBackend_OpenDal() when openDal != null:
return openDal(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  memory,TResult Function( String scheme,  Map<String, String> config)?  openDal,required TResult orElse(),}) {final _that = this;
switch (_that) {
case CacheBackend_Memory() when memory != null:
return memory();case CacheBackend_OpenDal() when openDal != null:
return openDal(_that.scheme,_that.config);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  memory,required TResult Function( String scheme,  Map<String, String> config)  openDal,}) {final _that = this;
switch (_that) {
case CacheBackend_Memory():
return memory();case CacheBackend_OpenDal():
return openDal(_that.scheme,_that.config);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  memory,TResult? Function( String scheme,  Map<String, String> config)?  openDal,}) {final _that = this;
switch (_that) {
case CacheBackend_Memory() when memory != null:
return memory();case CacheBackend_OpenDal() when openDal != null:
return openDal(_that.scheme,_that.config);case _:
  return null;

}
}

}

/// @nodoc


class CacheBackend_Memory extends CacheBackend {
  const CacheBackend_Memory(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CacheBackend_Memory);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'CacheBackend.memory()';
}


}




/// @nodoc


class CacheBackend_OpenDal extends CacheBackend {
  const CacheBackend_OpenDal({required this.scheme, required final  Map<String, String> config}): _config = config,super._();


/// OpenDAL scheme name (e.g. "s3", "redis", "fs", "gcs", "azblob").
 final  String scheme;
/// Backend-specific configuration as key-value pairs passed to OpenDAL.
 final  Map<String, String> _config;
/// Backend-specific configuration as key-value pairs passed to OpenDAL.
 Map<String, String> get config {
  if (_config is EqualUnmodifiableMapView) return _config;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableMapView(_config);
}


/// Create a copy of CacheBackend
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$CacheBackend_OpenDalCopyWith<CacheBackend_OpenDal> get copyWith => _$CacheBackend_OpenDalCopyWithImpl<CacheBackend_OpenDal>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is CacheBackend_OpenDal&&(identical(other.scheme, scheme) || other.scheme == scheme)&&const DeepCollectionEquality().equals(other._config, _config));
}


@override
int get hashCode => Object.hash(runtimeType,scheme,const DeepCollectionEquality().hash(_config));

@override
String toString() {
  return 'CacheBackend.openDal(scheme: $scheme, config: $config)';
}


}

/// @nodoc
abstract mixin class $CacheBackend_OpenDalCopyWith<$Res> implements $CacheBackendCopyWith<$Res> {
  factory $CacheBackend_OpenDalCopyWith(CacheBackend_OpenDal value, $Res Function(CacheBackend_OpenDal) _then) = _$CacheBackend_OpenDalCopyWithImpl;
@useResult
$Res call({
 String scheme, Map<String, String> config
});




}
/// @nodoc
class _$CacheBackend_OpenDalCopyWithImpl<$Res>
    implements $CacheBackend_OpenDalCopyWith<$Res> {
  _$CacheBackend_OpenDalCopyWithImpl(this._self, this._then);

  final CacheBackend_OpenDal _self;
  final $Res Function(CacheBackend_OpenDal) _then;

/// Create a copy of CacheBackend
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? scheme = null,Object? config = null,}) {
  return _then(CacheBackend_OpenDal(
scheme: null == scheme ? _self.scheme : scheme // ignore: cast_nullable_to_non_nullable
as String,config: null == config ? _self._config : config // ignore: cast_nullable_to_non_nullable
as Map<String, String>,
  ));
}


}

/// @nodoc
mixin _$ContentPart {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ContentPart);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ContentPart()';
}


}

/// @nodoc
class $ContentPartCopyWith<$Res>  {
$ContentPartCopyWith(ContentPart _, $Res Function(ContentPart) __);
}


/// Adds pattern-matching-related methods to [ContentPart].
extension ContentPartPatterns on ContentPart {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ContentPart_Text value)?  text,TResult Function( ContentPart_ImageUrl value)?  imageUrl,TResult Function( ContentPart_Document value)?  document,TResult Function( ContentPart_InputAudio value)?  inputAudio,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ContentPart_Text() when text != null:
return text(_that);case ContentPart_ImageUrl() when imageUrl != null:
return imageUrl(_that);case ContentPart_Document() when document != null:
return document(_that);case ContentPart_InputAudio() when inputAudio != null:
return inputAudio(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ContentPart_Text value)  text,required TResult Function( ContentPart_ImageUrl value)  imageUrl,required TResult Function( ContentPart_Document value)  document,required TResult Function( ContentPart_InputAudio value)  inputAudio,}){
final _that = this;
switch (_that) {
case ContentPart_Text():
return text(_that);case ContentPart_ImageUrl():
return imageUrl(_that);case ContentPart_Document():
return document(_that);case ContentPart_InputAudio():
return inputAudio(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ContentPart_Text value)?  text,TResult? Function( ContentPart_ImageUrl value)?  imageUrl,TResult? Function( ContentPart_Document value)?  document,TResult? Function( ContentPart_InputAudio value)?  inputAudio,}){
final _that = this;
switch (_that) {
case ContentPart_Text() when text != null:
return text(_that);case ContentPart_ImageUrl() when imageUrl != null:
return imageUrl(_that);case ContentPart_Document() when document != null:
return document(_that);case ContentPart_InputAudio() when inputAudio != null:
return inputAudio(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String text)?  text,TResult Function( ImageUrl imageUrl)?  imageUrl,TResult Function( DocumentContent document)?  document,TResult Function( AudioContent inputAudio)?  inputAudio,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ContentPart_Text() when text != null:
return text(_that.text);case ContentPart_ImageUrl() when imageUrl != null:
return imageUrl(_that.imageUrl);case ContentPart_Document() when document != null:
return document(_that.document);case ContentPart_InputAudio() when inputAudio != null:
return inputAudio(_that.inputAudio);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String text)  text,required TResult Function( ImageUrl imageUrl)  imageUrl,required TResult Function( DocumentContent document)  document,required TResult Function( AudioContent inputAudio)  inputAudio,}) {final _that = this;
switch (_that) {
case ContentPart_Text():
return text(_that.text);case ContentPart_ImageUrl():
return imageUrl(_that.imageUrl);case ContentPart_Document():
return document(_that.document);case ContentPart_InputAudio():
return inputAudio(_that.inputAudio);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String text)?  text,TResult? Function( ImageUrl imageUrl)?  imageUrl,TResult? Function( DocumentContent document)?  document,TResult? Function( AudioContent inputAudio)?  inputAudio,}) {final _that = this;
switch (_that) {
case ContentPart_Text() when text != null:
return text(_that.text);case ContentPart_ImageUrl() when imageUrl != null:
return imageUrl(_that.imageUrl);case ContentPart_Document() when document != null:
return document(_that.document);case ContentPart_InputAudio() when inputAudio != null:
return inputAudio(_that.inputAudio);case _:
  return null;

}
}

}

/// @nodoc


class ContentPart_Text extends ContentPart {
  const ContentPart_Text({required this.text}): super._();


 final  String text;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ContentPart_TextCopyWith<ContentPart_Text> get copyWith => _$ContentPart_TextCopyWithImpl<ContentPart_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ContentPart_Text&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode => Object.hash(runtimeType,text);

@override
String toString() {
  return 'ContentPart.text(text: $text)';
}


}

/// @nodoc
abstract mixin class $ContentPart_TextCopyWith<$Res> implements $ContentPartCopyWith<$Res> {
  factory $ContentPart_TextCopyWith(ContentPart_Text value, $Res Function(ContentPart_Text) _then) = _$ContentPart_TextCopyWithImpl;
@useResult
$Res call({
 String text
});




}
/// @nodoc
class _$ContentPart_TextCopyWithImpl<$Res>
    implements $ContentPart_TextCopyWith<$Res> {
  _$ContentPart_TextCopyWithImpl(this._self, this._then);

  final ContentPart_Text _self;
  final $Res Function(ContentPart_Text) _then;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,}) {
  return _then(ContentPart_Text(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class ContentPart_ImageUrl extends ContentPart {
  const ContentPart_ImageUrl({required this.imageUrl}): super._();


 final  ImageUrl imageUrl;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ContentPart_ImageUrlCopyWith<ContentPart_ImageUrl> get copyWith => _$ContentPart_ImageUrlCopyWithImpl<ContentPart_ImageUrl>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ContentPart_ImageUrl&&(identical(other.imageUrl, imageUrl) || other.imageUrl == imageUrl));
}


@override
int get hashCode => Object.hash(runtimeType,imageUrl);

@override
String toString() {
  return 'ContentPart.imageUrl(imageUrl: $imageUrl)';
}


}

/// @nodoc
abstract mixin class $ContentPart_ImageUrlCopyWith<$Res> implements $ContentPartCopyWith<$Res> {
  factory $ContentPart_ImageUrlCopyWith(ContentPart_ImageUrl value, $Res Function(ContentPart_ImageUrl) _then) = _$ContentPart_ImageUrlCopyWithImpl;
@useResult
$Res call({
 ImageUrl imageUrl
});




}
/// @nodoc
class _$ContentPart_ImageUrlCopyWithImpl<$Res>
    implements $ContentPart_ImageUrlCopyWith<$Res> {
  _$ContentPart_ImageUrlCopyWithImpl(this._self, this._then);

  final ContentPart_ImageUrl _self;
  final $Res Function(ContentPart_ImageUrl) _then;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? imageUrl = null,}) {
  return _then(ContentPart_ImageUrl(
imageUrl: null == imageUrl ? _self.imageUrl : imageUrl // ignore: cast_nullable_to_non_nullable
as ImageUrl,
  ));
}


}

/// @nodoc


class ContentPart_Document extends ContentPart {
  const ContentPart_Document({required this.document}): super._();


 final  DocumentContent document;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ContentPart_DocumentCopyWith<ContentPart_Document> get copyWith => _$ContentPart_DocumentCopyWithImpl<ContentPart_Document>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ContentPart_Document&&(identical(other.document, document) || other.document == document));
}


@override
int get hashCode => Object.hash(runtimeType,document);

@override
String toString() {
  return 'ContentPart.document(document: $document)';
}


}

/// @nodoc
abstract mixin class $ContentPart_DocumentCopyWith<$Res> implements $ContentPartCopyWith<$Res> {
  factory $ContentPart_DocumentCopyWith(ContentPart_Document value, $Res Function(ContentPart_Document) _then) = _$ContentPart_DocumentCopyWithImpl;
@useResult
$Res call({
 DocumentContent document
});




}
/// @nodoc
class _$ContentPart_DocumentCopyWithImpl<$Res>
    implements $ContentPart_DocumentCopyWith<$Res> {
  _$ContentPart_DocumentCopyWithImpl(this._self, this._then);

  final ContentPart_Document _self;
  final $Res Function(ContentPart_Document) _then;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? document = null,}) {
  return _then(ContentPart_Document(
document: null == document ? _self.document : document // ignore: cast_nullable_to_non_nullable
as DocumentContent,
  ));
}


}

/// @nodoc


class ContentPart_InputAudio extends ContentPart {
  const ContentPart_InputAudio({required this.inputAudio}): super._();


 final  AudioContent inputAudio;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ContentPart_InputAudioCopyWith<ContentPart_InputAudio> get copyWith => _$ContentPart_InputAudioCopyWithImpl<ContentPart_InputAudio>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ContentPart_InputAudio&&(identical(other.inputAudio, inputAudio) || other.inputAudio == inputAudio));
}


@override
int get hashCode => Object.hash(runtimeType,inputAudio);

@override
String toString() {
  return 'ContentPart.inputAudio(inputAudio: $inputAudio)';
}


}

/// @nodoc
abstract mixin class $ContentPart_InputAudioCopyWith<$Res> implements $ContentPartCopyWith<$Res> {
  factory $ContentPart_InputAudioCopyWith(ContentPart_InputAudio value, $Res Function(ContentPart_InputAudio) _then) = _$ContentPart_InputAudioCopyWithImpl;
@useResult
$Res call({
 AudioContent inputAudio
});




}
/// @nodoc
class _$ContentPart_InputAudioCopyWithImpl<$Res>
    implements $ContentPart_InputAudioCopyWith<$Res> {
  _$ContentPart_InputAudioCopyWithImpl(this._self, this._then);

  final ContentPart_InputAudio _self;
  final $Res Function(ContentPart_InputAudio) _then;

/// Create a copy of ContentPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? inputAudio = null,}) {
  return _then(ContentPart_InputAudio(
inputAudio: null == inputAudio ? _self.inputAudio : inputAudio // ignore: cast_nullable_to_non_nullable
as AudioContent,
  ));
}


}

/// @nodoc
mixin _$EmbeddingInput {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbeddingInput&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'EmbeddingInput(field0: $field0)';
}


}

/// @nodoc
class $EmbeddingInputCopyWith<$Res>  {
$EmbeddingInputCopyWith(EmbeddingInput _, $Res Function(EmbeddingInput) __);
}


/// Adds pattern-matching-related methods to [EmbeddingInput].
extension EmbeddingInputPatterns on EmbeddingInput {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( EmbeddingInput_Single value)?  single,TResult Function( EmbeddingInput_Multiple value)?  multiple,required TResult orElse(),}){
final _that = this;
switch (_that) {
case EmbeddingInput_Single() when single != null:
return single(_that);case EmbeddingInput_Multiple() when multiple != null:
return multiple(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( EmbeddingInput_Single value)  single,required TResult Function( EmbeddingInput_Multiple value)  multiple,}){
final _that = this;
switch (_that) {
case EmbeddingInput_Single():
return single(_that);case EmbeddingInput_Multiple():
return multiple(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( EmbeddingInput_Single value)?  single,TResult? Function( EmbeddingInput_Multiple value)?  multiple,}){
final _that = this;
switch (_that) {
case EmbeddingInput_Single() when single != null:
return single(_that);case EmbeddingInput_Multiple() when multiple != null:
return multiple(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  single,TResult Function( List<String> field0)?  multiple,required TResult orElse(),}) {final _that = this;
switch (_that) {
case EmbeddingInput_Single() when single != null:
return single(_that.field0);case EmbeddingInput_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  single,required TResult Function( List<String> field0)  multiple,}) {final _that = this;
switch (_that) {
case EmbeddingInput_Single():
return single(_that.field0);case EmbeddingInput_Multiple():
return multiple(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  single,TResult? Function( List<String> field0)?  multiple,}) {final _that = this;
switch (_that) {
case EmbeddingInput_Single() when single != null:
return single(_that.field0);case EmbeddingInput_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class EmbeddingInput_Single extends EmbeddingInput {
  const EmbeddingInput_Single({required this.field0}): super._();


@override final  String field0;

/// Create a copy of EmbeddingInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EmbeddingInput_SingleCopyWith<EmbeddingInput_Single> get copyWith => _$EmbeddingInput_SingleCopyWithImpl<EmbeddingInput_Single>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbeddingInput_Single&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'EmbeddingInput.single(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $EmbeddingInput_SingleCopyWith<$Res> implements $EmbeddingInputCopyWith<$Res> {
  factory $EmbeddingInput_SingleCopyWith(EmbeddingInput_Single value, $Res Function(EmbeddingInput_Single) _then) = _$EmbeddingInput_SingleCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$EmbeddingInput_SingleCopyWithImpl<$Res>
    implements $EmbeddingInput_SingleCopyWith<$Res> {
  _$EmbeddingInput_SingleCopyWithImpl(this._self, this._then);

  final EmbeddingInput_Single _self;
  final $Res Function(EmbeddingInput_Single) _then;

/// Create a copy of EmbeddingInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(EmbeddingInput_Single(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class EmbeddingInput_Multiple extends EmbeddingInput {
  const EmbeddingInput_Multiple({required final  List<String> field0}): _field0 = field0,super._();


 final  List<String> _field0;
@override List<String> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of EmbeddingInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EmbeddingInput_MultipleCopyWith<EmbeddingInput_Multiple> get copyWith => _$EmbeddingInput_MultipleCopyWithImpl<EmbeddingInput_Multiple>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbeddingInput_Multiple&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'EmbeddingInput.multiple(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $EmbeddingInput_MultipleCopyWith<$Res> implements $EmbeddingInputCopyWith<$Res> {
  factory $EmbeddingInput_MultipleCopyWith(EmbeddingInput_Multiple value, $Res Function(EmbeddingInput_Multiple) _then) = _$EmbeddingInput_MultipleCopyWithImpl;
@useResult
$Res call({
 List<String> field0
});




}
/// @nodoc
class _$EmbeddingInput_MultipleCopyWithImpl<$Res>
    implements $EmbeddingInput_MultipleCopyWith<$Res> {
  _$EmbeddingInput_MultipleCopyWithImpl(this._self, this._then);

  final EmbeddingInput_Multiple _self;
  final $Res Function(EmbeddingInput_Multiple) _then;

/// Create a copy of EmbeddingInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(EmbeddingInput_Multiple(
field0: null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc
mixin _$Message {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'Message(field0: $field0)';
}


}

/// @nodoc
class $MessageCopyWith<$Res>  {
$MessageCopyWith(Message _, $Res Function(Message) __);
}


/// Adds pattern-matching-related methods to [Message].
extension MessagePatterns on Message {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Message_System value)?  system,TResult Function( Message_User value)?  user,TResult Function( Message_Assistant value)?  assistant,TResult Function( Message_Tool value)?  tool,TResult Function( Message_Developer value)?  developer,TResult Function( Message_Function value)?  function,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Message_System() when system != null:
return system(_that);case Message_User() when user != null:
return user(_that);case Message_Assistant() when assistant != null:
return assistant(_that);case Message_Tool() when tool != null:
return tool(_that);case Message_Developer() when developer != null:
return developer(_that);case Message_Function() when function != null:
return function(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Message_System value)  system,required TResult Function( Message_User value)  user,required TResult Function( Message_Assistant value)  assistant,required TResult Function( Message_Tool value)  tool,required TResult Function( Message_Developer value)  developer,required TResult Function( Message_Function value)  function,}){
final _that = this;
switch (_that) {
case Message_System():
return system(_that);case Message_User():
return user(_that);case Message_Assistant():
return assistant(_that);case Message_Tool():
return tool(_that);case Message_Developer():
return developer(_that);case Message_Function():
return function(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Message_System value)?  system,TResult? Function( Message_User value)?  user,TResult? Function( Message_Assistant value)?  assistant,TResult? Function( Message_Tool value)?  tool,TResult? Function( Message_Developer value)?  developer,TResult? Function( Message_Function value)?  function,}){
final _that = this;
switch (_that) {
case Message_System() when system != null:
return system(_that);case Message_User() when user != null:
return user(_that);case Message_Assistant() when assistant != null:
return assistant(_that);case Message_Tool() when tool != null:
return tool(_that);case Message_Developer() when developer != null:
return developer(_that);case Message_Function() when function != null:
return function(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( SystemMessage field0)?  system,TResult Function( UserMessage field0)?  user,TResult Function( AssistantMessage field0)?  assistant,TResult Function( ToolMessage field0)?  tool,TResult Function( DeveloperMessage field0)?  developer,TResult Function( FunctionMessage field0)?  function,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Message_System() when system != null:
return system(_that.field0);case Message_User() when user != null:
return user(_that.field0);case Message_Assistant() when assistant != null:
return assistant(_that.field0);case Message_Tool() when tool != null:
return tool(_that.field0);case Message_Developer() when developer != null:
return developer(_that.field0);case Message_Function() when function != null:
return function(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( SystemMessage field0)  system,required TResult Function( UserMessage field0)  user,required TResult Function( AssistantMessage field0)  assistant,required TResult Function( ToolMessage field0)  tool,required TResult Function( DeveloperMessage field0)  developer,required TResult Function( FunctionMessage field0)  function,}) {final _that = this;
switch (_that) {
case Message_System():
return system(_that.field0);case Message_User():
return user(_that.field0);case Message_Assistant():
return assistant(_that.field0);case Message_Tool():
return tool(_that.field0);case Message_Developer():
return developer(_that.field0);case Message_Function():
return function(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( SystemMessage field0)?  system,TResult? Function( UserMessage field0)?  user,TResult? Function( AssistantMessage field0)?  assistant,TResult? Function( ToolMessage field0)?  tool,TResult? Function( DeveloperMessage field0)?  developer,TResult? Function( FunctionMessage field0)?  function,}) {final _that = this;
switch (_that) {
case Message_System() when system != null:
return system(_that.field0);case Message_User() when user != null:
return user(_that.field0);case Message_Assistant() when assistant != null:
return assistant(_that.field0);case Message_Tool() when tool != null:
return tool(_that.field0);case Message_Developer() when developer != null:
return developer(_that.field0);case Message_Function() when function != null:
return function(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class Message_System extends Message {
  const Message_System({required this.field0}): super._();


@override final  SystemMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_SystemCopyWith<Message_System> get copyWith => _$Message_SystemCopyWithImpl<Message_System>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_System&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.system(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_SystemCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_SystemCopyWith(Message_System value, $Res Function(Message_System) _then) = _$Message_SystemCopyWithImpl;
@useResult
$Res call({
 SystemMessage field0
});




}
/// @nodoc
class _$Message_SystemCopyWithImpl<$Res>
    implements $Message_SystemCopyWith<$Res> {
  _$Message_SystemCopyWithImpl(this._self, this._then);

  final Message_System _self;
  final $Res Function(Message_System) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_System(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SystemMessage,
  ));
}


}

/// @nodoc


class Message_User extends Message {
  const Message_User({required this.field0}): super._();


@override final  UserMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_UserCopyWith<Message_User> get copyWith => _$Message_UserCopyWithImpl<Message_User>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_User&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.user(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_UserCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_UserCopyWith(Message_User value, $Res Function(Message_User) _then) = _$Message_UserCopyWithImpl;
@useResult
$Res call({
 UserMessage field0
});




}
/// @nodoc
class _$Message_UserCopyWithImpl<$Res>
    implements $Message_UserCopyWith<$Res> {
  _$Message_UserCopyWithImpl(this._self, this._then);

  final Message_User _self;
  final $Res Function(Message_User) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_User(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UserMessage,
  ));
}


}

/// @nodoc


class Message_Assistant extends Message {
  const Message_Assistant({required this.field0}): super._();


@override final  AssistantMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_AssistantCopyWith<Message_Assistant> get copyWith => _$Message_AssistantCopyWithImpl<Message_Assistant>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_Assistant&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.assistant(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_AssistantCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_AssistantCopyWith(Message_Assistant value, $Res Function(Message_Assistant) _then) = _$Message_AssistantCopyWithImpl;
@useResult
$Res call({
 AssistantMessage field0
});




}
/// @nodoc
class _$Message_AssistantCopyWithImpl<$Res>
    implements $Message_AssistantCopyWith<$Res> {
  _$Message_AssistantCopyWithImpl(this._self, this._then);

  final Message_Assistant _self;
  final $Res Function(Message_Assistant) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_Assistant(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as AssistantMessage,
  ));
}


}

/// @nodoc


class Message_Tool extends Message {
  const Message_Tool({required this.field0}): super._();


@override final  ToolMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_ToolCopyWith<Message_Tool> get copyWith => _$Message_ToolCopyWithImpl<Message_Tool>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_Tool&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.tool(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_ToolCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_ToolCopyWith(Message_Tool value, $Res Function(Message_Tool) _then) = _$Message_ToolCopyWithImpl;
@useResult
$Res call({
 ToolMessage field0
});




}
/// @nodoc
class _$Message_ToolCopyWithImpl<$Res>
    implements $Message_ToolCopyWith<$Res> {
  _$Message_ToolCopyWithImpl(this._self, this._then);

  final Message_Tool _self;
  final $Res Function(Message_Tool) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_Tool(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToolMessage,
  ));
}


}

/// @nodoc


class Message_Developer extends Message {
  const Message_Developer({required this.field0}): super._();


@override final  DeveloperMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_DeveloperCopyWith<Message_Developer> get copyWith => _$Message_DeveloperCopyWithImpl<Message_Developer>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_Developer&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.developer(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_DeveloperCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_DeveloperCopyWith(Message_Developer value, $Res Function(Message_Developer) _then) = _$Message_DeveloperCopyWithImpl;
@useResult
$Res call({
 DeveloperMessage field0
});




}
/// @nodoc
class _$Message_DeveloperCopyWithImpl<$Res>
    implements $Message_DeveloperCopyWith<$Res> {
  _$Message_DeveloperCopyWithImpl(this._self, this._then);

  final Message_Developer _self;
  final $Res Function(Message_Developer) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_Developer(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DeveloperMessage,
  ));
}


}

/// @nodoc


class Message_Function extends Message {
  const Message_Function({required this.field0}): super._();


@override final  FunctionMessage field0;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Message_FunctionCopyWith<Message_Function> get copyWith => _$Message_FunctionCopyWithImpl<Message_Function>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Message_Function&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'Message.function(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Message_FunctionCopyWith<$Res> implements $MessageCopyWith<$Res> {
  factory $Message_FunctionCopyWith(Message_Function value, $Res Function(Message_Function) _then) = _$Message_FunctionCopyWithImpl;
@useResult
$Res call({
 FunctionMessage field0
});




}
/// @nodoc
class _$Message_FunctionCopyWithImpl<$Res>
    implements $Message_FunctionCopyWith<$Res> {
  _$Message_FunctionCopyWithImpl(this._self, this._then);

  final Message_Function _self;
  final $Res Function(Message_Function) _then;

/// Create a copy of Message
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Message_Function(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as FunctionMessage,
  ));
}


}

/// @nodoc
mixin _$ModerationInput {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModerationInput&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'ModerationInput(field0: $field0)';
}


}

/// @nodoc
class $ModerationInputCopyWith<$Res>  {
$ModerationInputCopyWith(ModerationInput _, $Res Function(ModerationInput) __);
}


/// Adds pattern-matching-related methods to [ModerationInput].
extension ModerationInputPatterns on ModerationInput {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ModerationInput_Single value)?  single,TResult Function( ModerationInput_Multiple value)?  multiple,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ModerationInput_Single() when single != null:
return single(_that);case ModerationInput_Multiple() when multiple != null:
return multiple(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ModerationInput_Single value)  single,required TResult Function( ModerationInput_Multiple value)  multiple,}){
final _that = this;
switch (_that) {
case ModerationInput_Single():
return single(_that);case ModerationInput_Multiple():
return multiple(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ModerationInput_Single value)?  single,TResult? Function( ModerationInput_Multiple value)?  multiple,}){
final _that = this;
switch (_that) {
case ModerationInput_Single() when single != null:
return single(_that);case ModerationInput_Multiple() when multiple != null:
return multiple(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  single,TResult Function( List<String> field0)?  multiple,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ModerationInput_Single() when single != null:
return single(_that.field0);case ModerationInput_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  single,required TResult Function( List<String> field0)  multiple,}) {final _that = this;
switch (_that) {
case ModerationInput_Single():
return single(_that.field0);case ModerationInput_Multiple():
return multiple(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  single,TResult? Function( List<String> field0)?  multiple,}) {final _that = this;
switch (_that) {
case ModerationInput_Single() when single != null:
return single(_that.field0);case ModerationInput_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class ModerationInput_Single extends ModerationInput {
  const ModerationInput_Single({required this.field0}): super._();


@override final  String field0;

/// Create a copy of ModerationInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ModerationInput_SingleCopyWith<ModerationInput_Single> get copyWith => _$ModerationInput_SingleCopyWithImpl<ModerationInput_Single>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModerationInput_Single&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ModerationInput.single(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ModerationInput_SingleCopyWith<$Res> implements $ModerationInputCopyWith<$Res> {
  factory $ModerationInput_SingleCopyWith(ModerationInput_Single value, $Res Function(ModerationInput_Single) _then) = _$ModerationInput_SingleCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$ModerationInput_SingleCopyWithImpl<$Res>
    implements $ModerationInput_SingleCopyWith<$Res> {
  _$ModerationInput_SingleCopyWithImpl(this._self, this._then);

  final ModerationInput_Single _self;
  final $Res Function(ModerationInput_Single) _then;

/// Create a copy of ModerationInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ModerationInput_Single(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class ModerationInput_Multiple extends ModerationInput {
  const ModerationInput_Multiple({required final  List<String> field0}): _field0 = field0,super._();


 final  List<String> _field0;
@override List<String> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of ModerationInput
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ModerationInput_MultipleCopyWith<ModerationInput_Multiple> get copyWith => _$ModerationInput_MultipleCopyWithImpl<ModerationInput_Multiple>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModerationInput_Multiple&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'ModerationInput.multiple(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ModerationInput_MultipleCopyWith<$Res> implements $ModerationInputCopyWith<$Res> {
  factory $ModerationInput_MultipleCopyWith(ModerationInput_Multiple value, $Res Function(ModerationInput_Multiple) _then) = _$ModerationInput_MultipleCopyWithImpl;
@useResult
$Res call({
 List<String> field0
});




}
/// @nodoc
class _$ModerationInput_MultipleCopyWithImpl<$Res>
    implements $ModerationInput_MultipleCopyWith<$Res> {
  _$ModerationInput_MultipleCopyWithImpl(this._self, this._then);

  final ModerationInput_Multiple _self;
  final $Res Function(ModerationInput_Multiple) _then;

/// Create a copy of ModerationInput
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ModerationInput_Multiple(
field0: null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc
mixin _$OcrDocument {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OcrDocument);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'OcrDocument()';
}


}

/// @nodoc
class $OcrDocumentCopyWith<$Res>  {
$OcrDocumentCopyWith(OcrDocument _, $Res Function(OcrDocument) __);
}


/// Adds pattern-matching-related methods to [OcrDocument].
extension OcrDocumentPatterns on OcrDocument {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( OcrDocument_Url value)?  url,TResult Function( OcrDocument_Base64 value)?  base64,required TResult orElse(),}){
final _that = this;
switch (_that) {
case OcrDocument_Url() when url != null:
return url(_that);case OcrDocument_Base64() when base64 != null:
return base64(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( OcrDocument_Url value)  url,required TResult Function( OcrDocument_Base64 value)  base64,}){
final _that = this;
switch (_that) {
case OcrDocument_Url():
return url(_that);case OcrDocument_Base64():
return base64(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( OcrDocument_Url value)?  url,TResult? Function( OcrDocument_Base64 value)?  base64,}){
final _that = this;
switch (_that) {
case OcrDocument_Url() when url != null:
return url(_that);case OcrDocument_Base64() when base64 != null:
return base64(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String url)?  url,TResult Function( String data,  String mediaType)?  base64,required TResult orElse(),}) {final _that = this;
switch (_that) {
case OcrDocument_Url() when url != null:
return url(_that.url);case OcrDocument_Base64() when base64 != null:
return base64(_that.data,_that.mediaType);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String url)  url,required TResult Function( String data,  String mediaType)  base64,}) {final _that = this;
switch (_that) {
case OcrDocument_Url():
return url(_that.url);case OcrDocument_Base64():
return base64(_that.data,_that.mediaType);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String url)?  url,TResult? Function( String data,  String mediaType)?  base64,}) {final _that = this;
switch (_that) {
case OcrDocument_Url() when url != null:
return url(_that.url);case OcrDocument_Base64() when base64 != null:
return base64(_that.data,_that.mediaType);case _:
  return null;

}
}

}

/// @nodoc


class OcrDocument_Url extends OcrDocument {
  const OcrDocument_Url({required this.url}): super._();


/// The document URL.
 final  String url;

/// Create a copy of OcrDocument
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OcrDocument_UrlCopyWith<OcrDocument_Url> get copyWith => _$OcrDocument_UrlCopyWithImpl<OcrDocument_Url>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OcrDocument_Url&&(identical(other.url, url) || other.url == url));
}


@override
int get hashCode => Object.hash(runtimeType,url);

@override
String toString() {
  return 'OcrDocument.url(url: $url)';
}


}

/// @nodoc
abstract mixin class $OcrDocument_UrlCopyWith<$Res> implements $OcrDocumentCopyWith<$Res> {
  factory $OcrDocument_UrlCopyWith(OcrDocument_Url value, $Res Function(OcrDocument_Url) _then) = _$OcrDocument_UrlCopyWithImpl;
@useResult
$Res call({
 String url
});




}
/// @nodoc
class _$OcrDocument_UrlCopyWithImpl<$Res>
    implements $OcrDocument_UrlCopyWith<$Res> {
  _$OcrDocument_UrlCopyWithImpl(this._self, this._then);

  final OcrDocument_Url _self;
  final $Res Function(OcrDocument_Url) _then;

/// Create a copy of OcrDocument
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,}) {
  return _then(OcrDocument_Url(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class OcrDocument_Base64 extends OcrDocument {
  const OcrDocument_Base64({required this.data, required this.mediaType}): super._();


/// Base64-encoded document content.
 final  String data;
/// MIME type (e.g. `"application/pdf"`, `"image/png"`).
 final  String mediaType;

/// Create a copy of OcrDocument
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$OcrDocument_Base64CopyWith<OcrDocument_Base64> get copyWith => _$OcrDocument_Base64CopyWithImpl<OcrDocument_Base64>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is OcrDocument_Base64&&(identical(other.data, data) || other.data == data)&&(identical(other.mediaType, mediaType) || other.mediaType == mediaType));
}


@override
int get hashCode => Object.hash(runtimeType,data,mediaType);

@override
String toString() {
  return 'OcrDocument.base64(data: $data, mediaType: $mediaType)';
}


}

/// @nodoc
abstract mixin class $OcrDocument_Base64CopyWith<$Res> implements $OcrDocumentCopyWith<$Res> {
  factory $OcrDocument_Base64CopyWith(OcrDocument_Base64 value, $Res Function(OcrDocument_Base64) _then) = _$OcrDocument_Base64CopyWithImpl;
@useResult
$Res call({
 String data, String mediaType
});




}
/// @nodoc
class _$OcrDocument_Base64CopyWithImpl<$Res>
    implements $OcrDocument_Base64CopyWith<$Res> {
  _$OcrDocument_Base64CopyWithImpl(this._self, this._then);

  final OcrDocument_Base64 _self;
  final $Res Function(OcrDocument_Base64) _then;

/// Create a copy of OcrDocument
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? data = null,Object? mediaType = null,}) {
  return _then(OcrDocument_Base64(
data: null == data ? _self.data : data // ignore: cast_nullable_to_non_nullable
as String,mediaType: null == mediaType ? _self.mediaType : mediaType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$RerankDocument {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RerankDocument);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'RerankDocument()';
}


}

/// @nodoc
class $RerankDocumentCopyWith<$Res>  {
$RerankDocumentCopyWith(RerankDocument _, $Res Function(RerankDocument) __);
}


/// Adds pattern-matching-related methods to [RerankDocument].
extension RerankDocumentPatterns on RerankDocument {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( RerankDocument_Text value)?  text,TResult Function( RerankDocument_Object value)?  object,required TResult orElse(),}){
final _that = this;
switch (_that) {
case RerankDocument_Text() when text != null:
return text(_that);case RerankDocument_Object() when object != null:
return object(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( RerankDocument_Text value)  text,required TResult Function( RerankDocument_Object value)  object,}){
final _that = this;
switch (_that) {
case RerankDocument_Text():
return text(_that);case RerankDocument_Object():
return object(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( RerankDocument_Text value)?  text,TResult? Function( RerankDocument_Object value)?  object,}){
final _that = this;
switch (_that) {
case RerankDocument_Text() when text != null:
return text(_that);case RerankDocument_Object() when object != null:
return object(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  text,TResult Function( String text)?  object,required TResult orElse(),}) {final _that = this;
switch (_that) {
case RerankDocument_Text() when text != null:
return text(_that.field0);case RerankDocument_Object() when object != null:
return object(_that.text);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  text,required TResult Function( String text)  object,}) {final _that = this;
switch (_that) {
case RerankDocument_Text():
return text(_that.field0);case RerankDocument_Object():
return object(_that.text);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  text,TResult? Function( String text)?  object,}) {final _that = this;
switch (_that) {
case RerankDocument_Text() when text != null:
return text(_that.field0);case RerankDocument_Object() when object != null:
return object(_that.text);case _:
  return null;

}
}

}

/// @nodoc


class RerankDocument_Text extends RerankDocument {
  const RerankDocument_Text({required this.field0}): super._();


 final  String field0;

/// Create a copy of RerankDocument
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$RerankDocument_TextCopyWith<RerankDocument_Text> get copyWith => _$RerankDocument_TextCopyWithImpl<RerankDocument_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RerankDocument_Text&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'RerankDocument.text(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $RerankDocument_TextCopyWith<$Res> implements $RerankDocumentCopyWith<$Res> {
  factory $RerankDocument_TextCopyWith(RerankDocument_Text value, $Res Function(RerankDocument_Text) _then) = _$RerankDocument_TextCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$RerankDocument_TextCopyWithImpl<$Res>
    implements $RerankDocument_TextCopyWith<$Res> {
  _$RerankDocument_TextCopyWithImpl(this._self, this._then);

  final RerankDocument_Text _self;
  final $Res Function(RerankDocument_Text) _then;

/// Create a copy of RerankDocument
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(RerankDocument_Text(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class RerankDocument_Object extends RerankDocument {
  const RerankDocument_Object({required this.text}): super._();


 final  String text;

/// Create a copy of RerankDocument
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$RerankDocument_ObjectCopyWith<RerankDocument_Object> get copyWith => _$RerankDocument_ObjectCopyWithImpl<RerankDocument_Object>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is RerankDocument_Object&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode => Object.hash(runtimeType,text);

@override
String toString() {
  return 'RerankDocument.object(text: $text)';
}


}

/// @nodoc
abstract mixin class $RerankDocument_ObjectCopyWith<$Res> implements $RerankDocumentCopyWith<$Res> {
  factory $RerankDocument_ObjectCopyWith(RerankDocument_Object value, $Res Function(RerankDocument_Object) _then) = _$RerankDocument_ObjectCopyWithImpl;
@useResult
$Res call({
 String text
});




}
/// @nodoc
class _$RerankDocument_ObjectCopyWithImpl<$Res>
    implements $RerankDocument_ObjectCopyWith<$Res> {
  _$RerankDocument_ObjectCopyWithImpl(this._self, this._then);

  final RerankDocument_Object _self;
  final $Res Function(RerankDocument_Object) _then;

/// Create a copy of RerankDocument
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,}) {
  return _then(RerankDocument_Object(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$ResponseFormat {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ResponseFormat);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ResponseFormat()';
}


}

/// @nodoc
class $ResponseFormatCopyWith<$Res>  {
$ResponseFormatCopyWith(ResponseFormat _, $Res Function(ResponseFormat) __);
}


/// Adds pattern-matching-related methods to [ResponseFormat].
extension ResponseFormatPatterns on ResponseFormat {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ResponseFormat_Text value)?  text,TResult Function( ResponseFormat_JsonObject value)?  jsonObject,TResult Function( ResponseFormat_JsonSchema value)?  jsonSchema,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ResponseFormat_Text() when text != null:
return text(_that);case ResponseFormat_JsonObject() when jsonObject != null:
return jsonObject(_that);case ResponseFormat_JsonSchema() when jsonSchema != null:
return jsonSchema(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ResponseFormat_Text value)  text,required TResult Function( ResponseFormat_JsonObject value)  jsonObject,required TResult Function( ResponseFormat_JsonSchema value)  jsonSchema,}){
final _that = this;
switch (_that) {
case ResponseFormat_Text():
return text(_that);case ResponseFormat_JsonObject():
return jsonObject(_that);case ResponseFormat_JsonSchema():
return jsonSchema(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ResponseFormat_Text value)?  text,TResult? Function( ResponseFormat_JsonObject value)?  jsonObject,TResult? Function( ResponseFormat_JsonSchema value)?  jsonSchema,}){
final _that = this;
switch (_that) {
case ResponseFormat_Text() when text != null:
return text(_that);case ResponseFormat_JsonObject() when jsonObject != null:
return jsonObject(_that);case ResponseFormat_JsonSchema() when jsonSchema != null:
return jsonSchema(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  text,TResult Function()?  jsonObject,TResult Function( JsonSchemaFormat jsonSchema)?  jsonSchema,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ResponseFormat_Text() when text != null:
return text();case ResponseFormat_JsonObject() when jsonObject != null:
return jsonObject();case ResponseFormat_JsonSchema() when jsonSchema != null:
return jsonSchema(_that.jsonSchema);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  text,required TResult Function()  jsonObject,required TResult Function( JsonSchemaFormat jsonSchema)  jsonSchema,}) {final _that = this;
switch (_that) {
case ResponseFormat_Text():
return text();case ResponseFormat_JsonObject():
return jsonObject();case ResponseFormat_JsonSchema():
return jsonSchema(_that.jsonSchema);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  text,TResult? Function()?  jsonObject,TResult? Function( JsonSchemaFormat jsonSchema)?  jsonSchema,}) {final _that = this;
switch (_that) {
case ResponseFormat_Text() when text != null:
return text();case ResponseFormat_JsonObject() when jsonObject != null:
return jsonObject();case ResponseFormat_JsonSchema() when jsonSchema != null:
return jsonSchema(_that.jsonSchema);case _:
  return null;

}
}

}

/// @nodoc


class ResponseFormat_Text extends ResponseFormat {
  const ResponseFormat_Text(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ResponseFormat_Text);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ResponseFormat.text()';
}


}




/// @nodoc


class ResponseFormat_JsonObject extends ResponseFormat {
  const ResponseFormat_JsonObject(): super._();







@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ResponseFormat_JsonObject);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ResponseFormat.jsonObject()';
}


}




/// @nodoc


class ResponseFormat_JsonSchema extends ResponseFormat {
  const ResponseFormat_JsonSchema({required this.jsonSchema}): super._();


 final  JsonSchemaFormat jsonSchema;

/// Create a copy of ResponseFormat
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ResponseFormat_JsonSchemaCopyWith<ResponseFormat_JsonSchema> get copyWith => _$ResponseFormat_JsonSchemaCopyWithImpl<ResponseFormat_JsonSchema>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ResponseFormat_JsonSchema&&(identical(other.jsonSchema, jsonSchema) || other.jsonSchema == jsonSchema));
}


@override
int get hashCode => Object.hash(runtimeType,jsonSchema);

@override
String toString() {
  return 'ResponseFormat.jsonSchema(jsonSchema: $jsonSchema)';
}


}

/// @nodoc
abstract mixin class $ResponseFormat_JsonSchemaCopyWith<$Res> implements $ResponseFormatCopyWith<$Res> {
  factory $ResponseFormat_JsonSchemaCopyWith(ResponseFormat_JsonSchema value, $Res Function(ResponseFormat_JsonSchema) _then) = _$ResponseFormat_JsonSchemaCopyWithImpl;
@useResult
$Res call({
 JsonSchemaFormat jsonSchema
});




}
/// @nodoc
class _$ResponseFormat_JsonSchemaCopyWithImpl<$Res>
    implements $ResponseFormat_JsonSchemaCopyWith<$Res> {
  _$ResponseFormat_JsonSchemaCopyWithImpl(this._self, this._then);

  final ResponseFormat_JsonSchema _self;
  final $Res Function(ResponseFormat_JsonSchema) _then;

/// Create a copy of ResponseFormat
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? jsonSchema = null,}) {
  return _then(ResponseFormat_JsonSchema(
jsonSchema: null == jsonSchema ? _self.jsonSchema : jsonSchema // ignore: cast_nullable_to_non_nullable
as JsonSchemaFormat,
  ));
}


}

/// @nodoc
mixin _$StopSequence {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StopSequence&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'StopSequence(field0: $field0)';
}


}

/// @nodoc
class $StopSequenceCopyWith<$Res>  {
$StopSequenceCopyWith(StopSequence _, $Res Function(StopSequence) __);
}


/// Adds pattern-matching-related methods to [StopSequence].
extension StopSequencePatterns on StopSequence {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( StopSequence_Single value)?  single,TResult Function( StopSequence_Multiple value)?  multiple,required TResult orElse(),}){
final _that = this;
switch (_that) {
case StopSequence_Single() when single != null:
return single(_that);case StopSequence_Multiple() when multiple != null:
return multiple(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( StopSequence_Single value)  single,required TResult Function( StopSequence_Multiple value)  multiple,}){
final _that = this;
switch (_that) {
case StopSequence_Single():
return single(_that);case StopSequence_Multiple():
return multiple(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( StopSequence_Single value)?  single,TResult? Function( StopSequence_Multiple value)?  multiple,}){
final _that = this;
switch (_that) {
case StopSequence_Single() when single != null:
return single(_that);case StopSequence_Multiple() when multiple != null:
return multiple(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  single,TResult Function( List<String> field0)?  multiple,required TResult orElse(),}) {final _that = this;
switch (_that) {
case StopSequence_Single() when single != null:
return single(_that.field0);case StopSequence_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  single,required TResult Function( List<String> field0)  multiple,}) {final _that = this;
switch (_that) {
case StopSequence_Single():
return single(_that.field0);case StopSequence_Multiple():
return multiple(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  single,TResult? Function( List<String> field0)?  multiple,}) {final _that = this;
switch (_that) {
case StopSequence_Single() when single != null:
return single(_that.field0);case StopSequence_Multiple() when multiple != null:
return multiple(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class StopSequence_Single extends StopSequence {
  const StopSequence_Single({required this.field0}): super._();


@override final  String field0;

/// Create a copy of StopSequence
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StopSequence_SingleCopyWith<StopSequence_Single> get copyWith => _$StopSequence_SingleCopyWithImpl<StopSequence_Single>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StopSequence_Single&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StopSequence.single(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StopSequence_SingleCopyWith<$Res> implements $StopSequenceCopyWith<$Res> {
  factory $StopSequence_SingleCopyWith(StopSequence_Single value, $Res Function(StopSequence_Single) _then) = _$StopSequence_SingleCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$StopSequence_SingleCopyWithImpl<$Res>
    implements $StopSequence_SingleCopyWith<$Res> {
  _$StopSequence_SingleCopyWithImpl(this._self, this._then);

  final StopSequence_Single _self;
  final $Res Function(StopSequence_Single) _then;

/// Create a copy of StopSequence
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StopSequence_Single(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class StopSequence_Multiple extends StopSequence {
  const StopSequence_Multiple({required final  List<String> field0}): _field0 = field0,super._();


 final  List<String> _field0;
@override List<String> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of StopSequence
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StopSequence_MultipleCopyWith<StopSequence_Multiple> get copyWith => _$StopSequence_MultipleCopyWithImpl<StopSequence_Multiple>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StopSequence_Multiple&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'StopSequence.multiple(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StopSequence_MultipleCopyWith<$Res> implements $StopSequenceCopyWith<$Res> {
  factory $StopSequence_MultipleCopyWith(StopSequence_Multiple value, $Res Function(StopSequence_Multiple) _then) = _$StopSequence_MultipleCopyWithImpl;
@useResult
$Res call({
 List<String> field0
});




}
/// @nodoc
class _$StopSequence_MultipleCopyWithImpl<$Res>
    implements $StopSequence_MultipleCopyWith<$Res> {
  _$StopSequence_MultipleCopyWithImpl(this._self, this._then);

  final StopSequence_Multiple _self;
  final $Res Function(StopSequence_Multiple) _then;

/// Create a copy of StopSequence
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StopSequence_Multiple(
field0: null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc
mixin _$ToolChoice {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ToolChoice&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'ToolChoice(field0: $field0)';
}


}

/// @nodoc
class $ToolChoiceCopyWith<$Res>  {
$ToolChoiceCopyWith(ToolChoice _, $Res Function(ToolChoice) __);
}


/// Adds pattern-matching-related methods to [ToolChoice].
extension ToolChoicePatterns on ToolChoice {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ToolChoice_Mode value)?  mode,TResult Function( ToolChoice_Specific value)?  specific,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ToolChoice_Mode() when mode != null:
return mode(_that);case ToolChoice_Specific() when specific != null:
return specific(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ToolChoice_Mode value)  mode,required TResult Function( ToolChoice_Specific value)  specific,}){
final _that = this;
switch (_that) {
case ToolChoice_Mode():
return mode(_that);case ToolChoice_Specific():
return specific(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ToolChoice_Mode value)?  mode,TResult? Function( ToolChoice_Specific value)?  specific,}){
final _that = this;
switch (_that) {
case ToolChoice_Mode() when mode != null:
return mode(_that);case ToolChoice_Specific() when specific != null:
return specific(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( ToolChoiceMode field0)?  mode,TResult Function( SpecificToolChoice field0)?  specific,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ToolChoice_Mode() when mode != null:
return mode(_that.field0);case ToolChoice_Specific() when specific != null:
return specific(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( ToolChoiceMode field0)  mode,required TResult Function( SpecificToolChoice field0)  specific,}) {final _that = this;
switch (_that) {
case ToolChoice_Mode():
return mode(_that.field0);case ToolChoice_Specific():
return specific(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( ToolChoiceMode field0)?  mode,TResult? Function( SpecificToolChoice field0)?  specific,}) {final _that = this;
switch (_that) {
case ToolChoice_Mode() when mode != null:
return mode(_that.field0);case ToolChoice_Specific() when specific != null:
return specific(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class ToolChoice_Mode extends ToolChoice {
  const ToolChoice_Mode({required this.field0}): super._();


@override final  ToolChoiceMode field0;

/// Create a copy of ToolChoice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ToolChoice_ModeCopyWith<ToolChoice_Mode> get copyWith => _$ToolChoice_ModeCopyWithImpl<ToolChoice_Mode>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ToolChoice_Mode&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ToolChoice.mode(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ToolChoice_ModeCopyWith<$Res> implements $ToolChoiceCopyWith<$Res> {
  factory $ToolChoice_ModeCopyWith(ToolChoice_Mode value, $Res Function(ToolChoice_Mode) _then) = _$ToolChoice_ModeCopyWithImpl;
@useResult
$Res call({
 ToolChoiceMode field0
});




}
/// @nodoc
class _$ToolChoice_ModeCopyWithImpl<$Res>
    implements $ToolChoice_ModeCopyWith<$Res> {
  _$ToolChoice_ModeCopyWithImpl(this._self, this._then);

  final ToolChoice_Mode _self;
  final $Res Function(ToolChoice_Mode) _then;

/// Create a copy of ToolChoice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ToolChoice_Mode(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ToolChoiceMode,
  ));
}


}

/// @nodoc


class ToolChoice_Specific extends ToolChoice {
  const ToolChoice_Specific({required this.field0}): super._();


@override final  SpecificToolChoice field0;

/// Create a copy of ToolChoice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ToolChoice_SpecificCopyWith<ToolChoice_Specific> get copyWith => _$ToolChoice_SpecificCopyWithImpl<ToolChoice_Specific>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ToolChoice_Specific&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'ToolChoice.specific(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $ToolChoice_SpecificCopyWith<$Res> implements $ToolChoiceCopyWith<$Res> {
  factory $ToolChoice_SpecificCopyWith(ToolChoice_Specific value, $Res Function(ToolChoice_Specific) _then) = _$ToolChoice_SpecificCopyWithImpl;
@useResult
$Res call({
 SpecificToolChoice field0
});




}
/// @nodoc
class _$ToolChoice_SpecificCopyWithImpl<$Res>
    implements $ToolChoice_SpecificCopyWith<$Res> {
  _$ToolChoice_SpecificCopyWithImpl(this._self, this._then);

  final ToolChoice_Specific _self;
  final $Res Function(ToolChoice_Specific) _then;

/// Create a copy of ToolChoice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(ToolChoice_Specific(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as SpecificToolChoice,
  ));
}


}

/// @nodoc
mixin _$UserContent {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UserContent&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'UserContent(field0: $field0)';
}


}

/// @nodoc
class $UserContentCopyWith<$Res>  {
$UserContentCopyWith(UserContent _, $Res Function(UserContent) __);
}


/// Adds pattern-matching-related methods to [UserContent].
extension UserContentPatterns on UserContent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UserContent_Text value)?  text,TResult Function( UserContent_Parts value)?  parts,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UserContent_Text() when text != null:
return text(_that);case UserContent_Parts() when parts != null:
return parts(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UserContent_Text value)  text,required TResult Function( UserContent_Parts value)  parts,}){
final _that = this;
switch (_that) {
case UserContent_Text():
return text(_that);case UserContent_Parts():
return parts(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UserContent_Text value)?  text,TResult? Function( UserContent_Parts value)?  parts,}){
final _that = this;
switch (_that) {
case UserContent_Text() when text != null:
return text(_that);case UserContent_Parts() when parts != null:
return parts(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  text,TResult Function( List<ContentPart> field0)?  parts,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UserContent_Text() when text != null:
return text(_that.field0);case UserContent_Parts() when parts != null:
return parts(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  text,required TResult Function( List<ContentPart> field0)  parts,}) {final _that = this;
switch (_that) {
case UserContent_Text():
return text(_that.field0);case UserContent_Parts():
return parts(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  text,TResult? Function( List<ContentPart> field0)?  parts,}) {final _that = this;
switch (_that) {
case UserContent_Text() when text != null:
return text(_that.field0);case UserContent_Parts() when parts != null:
return parts(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class UserContent_Text extends UserContent {
  const UserContent_Text({required this.field0}): super._();


@override final  String field0;

/// Create a copy of UserContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UserContent_TextCopyWith<UserContent_Text> get copyWith => _$UserContent_TextCopyWithImpl<UserContent_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UserContent_Text&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UserContent.text(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UserContent_TextCopyWith<$Res> implements $UserContentCopyWith<$Res> {
  factory $UserContent_TextCopyWith(UserContent_Text value, $Res Function(UserContent_Text) _then) = _$UserContent_TextCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$UserContent_TextCopyWithImpl<$Res>
    implements $UserContent_TextCopyWith<$Res> {
  _$UserContent_TextCopyWithImpl(this._self, this._then);

  final UserContent_Text _self;
  final $Res Function(UserContent_Text) _then;

/// Create a copy of UserContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UserContent_Text(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class UserContent_Parts extends UserContent {
  const UserContent_Parts({required final  List<ContentPart> field0}): _field0 = field0,super._();


 final  List<ContentPart> _field0;
@override List<ContentPart> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of UserContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UserContent_PartsCopyWith<UserContent_Parts> get copyWith => _$UserContent_PartsCopyWithImpl<UserContent_Parts>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UserContent_Parts&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'UserContent.parts(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UserContent_PartsCopyWith<$Res> implements $UserContentCopyWith<$Res> {
  factory $UserContent_PartsCopyWith(UserContent_Parts value, $Res Function(UserContent_Parts) _then) = _$UserContent_PartsCopyWithImpl;
@useResult
$Res call({
 List<ContentPart> field0
});




}
/// @nodoc
class _$UserContent_PartsCopyWithImpl<$Res>
    implements $UserContent_PartsCopyWith<$Res> {
  _$UserContent_PartsCopyWithImpl(this._self, this._then);

  final UserContent_Parts _self;
  final $Res Function(UserContent_Parts) _then;

/// Create a copy of UserContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UserContent_Parts(
field0: null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<ContentPart>,
  ));
}


}

// dart format on
