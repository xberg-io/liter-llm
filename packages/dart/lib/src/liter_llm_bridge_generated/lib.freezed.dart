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
mixin _$AssistantContent {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantContent&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'AssistantContent(field0: $field0)';
}


}

/// @nodoc
class $AssistantContentCopyWith<$Res>  {
$AssistantContentCopyWith(AssistantContent _, $Res Function(AssistantContent) __);
}


/// Adds pattern-matching-related methods to [AssistantContent].
extension AssistantContentPatterns on AssistantContent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AssistantContent_Text value)?  text,TResult Function( AssistantContent_Parts value)?  parts,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AssistantContent_Text() when text != null:
return text(_that);case AssistantContent_Parts() when parts != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AssistantContent_Text value)  text,required TResult Function( AssistantContent_Parts value)  parts,}){
final _that = this;
switch (_that) {
case AssistantContent_Text():
return text(_that);case AssistantContent_Parts():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AssistantContent_Text value)?  text,TResult? Function( AssistantContent_Parts value)?  parts,}){
final _that = this;
switch (_that) {
case AssistantContent_Text() when text != null:
return text(_that);case AssistantContent_Parts() when parts != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  text,TResult Function( List<AssistantPart> field0)?  parts,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AssistantContent_Text() when text != null:
return text(_that.field0);case AssistantContent_Parts() when parts != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  text,required TResult Function( List<AssistantPart> field0)  parts,}) {final _that = this;
switch (_that) {
case AssistantContent_Text():
return text(_that.field0);case AssistantContent_Parts():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  text,TResult? Function( List<AssistantPart> field0)?  parts,}) {final _that = this;
switch (_that) {
case AssistantContent_Text() when text != null:
return text(_that.field0);case AssistantContent_Parts() when parts != null:
return parts(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class AssistantContent_Text extends AssistantContent {
  const AssistantContent_Text({required this.field0}): super._();
  

@override final  String field0;

/// Create a copy of AssistantContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantContent_TextCopyWith<AssistantContent_Text> get copyWith => _$AssistantContent_TextCopyWithImpl<AssistantContent_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantContent_Text&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AssistantContent.text(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AssistantContent_TextCopyWith<$Res> implements $AssistantContentCopyWith<$Res> {
  factory $AssistantContent_TextCopyWith(AssistantContent_Text value, $Res Function(AssistantContent_Text) _then) = _$AssistantContent_TextCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$AssistantContent_TextCopyWithImpl<$Res>
    implements $AssistantContent_TextCopyWith<$Res> {
  _$AssistantContent_TextCopyWithImpl(this._self, this._then);

  final AssistantContent_Text _self;
  final $Res Function(AssistantContent_Text) _then;

/// Create a copy of AssistantContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AssistantContent_Text(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class AssistantContent_Parts extends AssistantContent {
  const AssistantContent_Parts({required final  List<AssistantPart> field0}): _field0 = field0,super._();
  

 final  List<AssistantPart> _field0;
@override List<AssistantPart> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of AssistantContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantContent_PartsCopyWith<AssistantContent_Parts> get copyWith => _$AssistantContent_PartsCopyWithImpl<AssistantContent_Parts>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantContent_Parts&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'AssistantContent.parts(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AssistantContent_PartsCopyWith<$Res> implements $AssistantContentCopyWith<$Res> {
  factory $AssistantContent_PartsCopyWith(AssistantContent_Parts value, $Res Function(AssistantContent_Parts) _then) = _$AssistantContent_PartsCopyWithImpl;
@useResult
$Res call({
 List<AssistantPart> field0
});




}
/// @nodoc
class _$AssistantContent_PartsCopyWithImpl<$Res>
    implements $AssistantContent_PartsCopyWith<$Res> {
  _$AssistantContent_PartsCopyWithImpl(this._self, this._then);

  final AssistantContent_Parts _self;
  final $Res Function(AssistantContent_Parts) _then;

/// Create a copy of AssistantContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AssistantContent_Parts(
field0: null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<AssistantPart>,
  ));
}


}

/// @nodoc
mixin _$AssistantPart {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantPart);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AssistantPart()';
}


}

/// @nodoc
class $AssistantPartCopyWith<$Res>  {
$AssistantPartCopyWith(AssistantPart _, $Res Function(AssistantPart) __);
}


/// Adds pattern-matching-related methods to [AssistantPart].
extension AssistantPartPatterns on AssistantPart {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AssistantPart_Text value)?  text,TResult Function( AssistantPart_Refusal value)?  refusal,TResult Function( AssistantPart_OutputImage value)?  outputImage,TResult Function( AssistantPart_OutputAudio value)?  outputAudio,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AssistantPart_Text() when text != null:
return text(_that);case AssistantPart_Refusal() when refusal != null:
return refusal(_that);case AssistantPart_OutputImage() when outputImage != null:
return outputImage(_that);case AssistantPart_OutputAudio() when outputAudio != null:
return outputAudio(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AssistantPart_Text value)  text,required TResult Function( AssistantPart_Refusal value)  refusal,required TResult Function( AssistantPart_OutputImage value)  outputImage,required TResult Function( AssistantPart_OutputAudio value)  outputAudio,}){
final _that = this;
switch (_that) {
case AssistantPart_Text():
return text(_that);case AssistantPart_Refusal():
return refusal(_that);case AssistantPart_OutputImage():
return outputImage(_that);case AssistantPart_OutputAudio():
return outputAudio(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AssistantPart_Text value)?  text,TResult? Function( AssistantPart_Refusal value)?  refusal,TResult? Function( AssistantPart_OutputImage value)?  outputImage,TResult? Function( AssistantPart_OutputAudio value)?  outputAudio,}){
final _that = this;
switch (_that) {
case AssistantPart_Text() when text != null:
return text(_that);case AssistantPart_Refusal() when refusal != null:
return refusal(_that);case AssistantPart_OutputImage() when outputImage != null:
return outputImage(_that);case AssistantPart_OutputAudio() when outputAudio != null:
return outputAudio(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String text)?  text,TResult Function( String refusal)?  refusal,TResult Function( ImageUrl imageUrl)?  outputImage,TResult Function( AudioContent audio)?  outputAudio,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AssistantPart_Text() when text != null:
return text(_that.text);case AssistantPart_Refusal() when refusal != null:
return refusal(_that.refusal);case AssistantPart_OutputImage() when outputImage != null:
return outputImage(_that.imageUrl);case AssistantPart_OutputAudio() when outputAudio != null:
return outputAudio(_that.audio);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String text)  text,required TResult Function( String refusal)  refusal,required TResult Function( ImageUrl imageUrl)  outputImage,required TResult Function( AudioContent audio)  outputAudio,}) {final _that = this;
switch (_that) {
case AssistantPart_Text():
return text(_that.text);case AssistantPart_Refusal():
return refusal(_that.refusal);case AssistantPart_OutputImage():
return outputImage(_that.imageUrl);case AssistantPart_OutputAudio():
return outputAudio(_that.audio);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String text)?  text,TResult? Function( String refusal)?  refusal,TResult? Function( ImageUrl imageUrl)?  outputImage,TResult? Function( AudioContent audio)?  outputAudio,}) {final _that = this;
switch (_that) {
case AssistantPart_Text() when text != null:
return text(_that.text);case AssistantPart_Refusal() when refusal != null:
return refusal(_that.refusal);case AssistantPart_OutputImage() when outputImage != null:
return outputImage(_that.imageUrl);case AssistantPart_OutputAudio() when outputAudio != null:
return outputAudio(_that.audio);case _:
  return null;

}
}

}

/// @nodoc


class AssistantPart_Text extends AssistantPart {
  const AssistantPart_Text({required this.text}): super._();
  

/// The text content of this part.
 final  String text;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantPart_TextCopyWith<AssistantPart_Text> get copyWith => _$AssistantPart_TextCopyWithImpl<AssistantPart_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantPart_Text&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode => Object.hash(runtimeType,text);

@override
String toString() {
  return 'AssistantPart.text(text: $text)';
}


}

/// @nodoc
abstract mixin class $AssistantPart_TextCopyWith<$Res> implements $AssistantPartCopyWith<$Res> {
  factory $AssistantPart_TextCopyWith(AssistantPart_Text value, $Res Function(AssistantPart_Text) _then) = _$AssistantPart_TextCopyWithImpl;
@useResult
$Res call({
 String text
});




}
/// @nodoc
class _$AssistantPart_TextCopyWithImpl<$Res>
    implements $AssistantPart_TextCopyWith<$Res> {
  _$AssistantPart_TextCopyWithImpl(this._self, this._then);

  final AssistantPart_Text _self;
  final $Res Function(AssistantPart_Text) _then;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,}) {
  return _then(AssistantPart_Text(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class AssistantPart_Refusal extends AssistantPart {
  const AssistantPart_Refusal({required this.refusal}): super._();
  

/// The refusal reason.
 final  String refusal;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantPart_RefusalCopyWith<AssistantPart_Refusal> get copyWith => _$AssistantPart_RefusalCopyWithImpl<AssistantPart_Refusal>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantPart_Refusal&&(identical(other.refusal, refusal) || other.refusal == refusal));
}


@override
int get hashCode => Object.hash(runtimeType,refusal);

@override
String toString() {
  return 'AssistantPart.refusal(refusal: $refusal)';
}


}

/// @nodoc
abstract mixin class $AssistantPart_RefusalCopyWith<$Res> implements $AssistantPartCopyWith<$Res> {
  factory $AssistantPart_RefusalCopyWith(AssistantPart_Refusal value, $Res Function(AssistantPart_Refusal) _then) = _$AssistantPart_RefusalCopyWithImpl;
@useResult
$Res call({
 String refusal
});




}
/// @nodoc
class _$AssistantPart_RefusalCopyWithImpl<$Res>
    implements $AssistantPart_RefusalCopyWith<$Res> {
  _$AssistantPart_RefusalCopyWithImpl(this._self, this._then);

  final AssistantPart_Refusal _self;
  final $Res Function(AssistantPart_Refusal) _then;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? refusal = null,}) {
  return _then(AssistantPart_Refusal(
refusal: null == refusal ? _self.refusal : refusal // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class AssistantPart_OutputImage extends AssistantPart {
  const AssistantPart_OutputImage({required this.imageUrl}): super._();
  

/// Image URL or data URI referencing the generated image.
 final  ImageUrl imageUrl;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantPart_OutputImageCopyWith<AssistantPart_OutputImage> get copyWith => _$AssistantPart_OutputImageCopyWithImpl<AssistantPart_OutputImage>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantPart_OutputImage&&(identical(other.imageUrl, imageUrl) || other.imageUrl == imageUrl));
}


@override
int get hashCode => Object.hash(runtimeType,imageUrl);

@override
String toString() {
  return 'AssistantPart.outputImage(imageUrl: $imageUrl)';
}


}

/// @nodoc
abstract mixin class $AssistantPart_OutputImageCopyWith<$Res> implements $AssistantPartCopyWith<$Res> {
  factory $AssistantPart_OutputImageCopyWith(AssistantPart_OutputImage value, $Res Function(AssistantPart_OutputImage) _then) = _$AssistantPart_OutputImageCopyWithImpl;
@useResult
$Res call({
 ImageUrl imageUrl
});




}
/// @nodoc
class _$AssistantPart_OutputImageCopyWithImpl<$Res>
    implements $AssistantPart_OutputImageCopyWith<$Res> {
  _$AssistantPart_OutputImageCopyWithImpl(this._self, this._then);

  final AssistantPart_OutputImage _self;
  final $Res Function(AssistantPart_OutputImage) _then;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? imageUrl = null,}) {
  return _then(AssistantPart_OutputImage(
imageUrl: null == imageUrl ? _self.imageUrl : imageUrl // ignore: cast_nullable_to_non_nullable
as ImageUrl,
  ));
}


}

/// @nodoc


class AssistantPart_OutputAudio extends AssistantPart {
  const AssistantPart_OutputAudio({required this.audio}): super._();
  

/// Base64-encoded audio data and its format.
 final  AudioContent audio;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AssistantPart_OutputAudioCopyWith<AssistantPart_OutputAudio> get copyWith => _$AssistantPart_OutputAudioCopyWithImpl<AssistantPart_OutputAudio>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AssistantPart_OutputAudio&&(identical(other.audio, audio) || other.audio == audio));
}


@override
int get hashCode => Object.hash(runtimeType,audio);

@override
String toString() {
  return 'AssistantPart.outputAudio(audio: $audio)';
}


}

/// @nodoc
abstract mixin class $AssistantPart_OutputAudioCopyWith<$Res> implements $AssistantPartCopyWith<$Res> {
  factory $AssistantPart_OutputAudioCopyWith(AssistantPart_OutputAudio value, $Res Function(AssistantPart_OutputAudio) _then) = _$AssistantPart_OutputAudioCopyWithImpl;
@useResult
$Res call({
 AudioContent audio
});




}
/// @nodoc
class _$AssistantPart_OutputAudioCopyWithImpl<$Res>
    implements $AssistantPart_OutputAudioCopyWith<$Res> {
  _$AssistantPart_OutputAudioCopyWithImpl(this._self, this._then);

  final AssistantPart_OutputAudio _self;
  final $Res Function(AssistantPart_OutputAudio) _then;

/// Create a copy of AssistantPart
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? audio = null,}) {
  return _then(AssistantPart_OutputAudio(
audio: null == audio ? _self.audio : audio // ignore: cast_nullable_to_non_nullable
as AudioContent,
  ));
}


}

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
mixin _$LiterLlmError {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'LiterLlmError()';
}


}

/// @nodoc
class $LiterLlmErrorCopyWith<$Res>  {
$LiterLlmErrorCopyWith(LiterLlmError _, $Res Function(LiterLlmError) __);
}


/// Adds pattern-matching-related methods to [LiterLlmError].
extension LiterLlmErrorPatterns on LiterLlmError {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( LiterLlmError_Authentication value)?  authentication,TResult Function( LiterLlmError_RateLimited value)?  rateLimited,TResult Function( LiterLlmError_BadRequest value)?  badRequest,TResult Function( LiterLlmError_ContextWindowExceeded value)?  contextWindowExceeded,TResult Function( LiterLlmError_ContentPolicy value)?  contentPolicy,TResult Function( LiterLlmError_NotFound value)?  notFound,TResult Function( LiterLlmError_ServerError value)?  serverError,TResult Function( LiterLlmError_ServiceUnavailable value)?  serviceUnavailable,TResult Function( LiterLlmError_Timeout value)?  timeout,TResult Function( LiterLlmError_Streaming value)?  streaming,TResult Function( LiterLlmError_EndpointNotSupported value)?  endpointNotSupported,TResult Function( LiterLlmError_InvalidHeader value)?  invalidHeader,TResult Function( LiterLlmError_Serialization value)?  serialization,TResult Function( LiterLlmError_BudgetExceeded value)?  budgetExceeded,TResult Function( LiterLlmError_HookRejected value)?  hookRejected,TResult Function( LiterLlmError_InternalError value)?  internalError,TResult Function( LiterLlmError_OutboundForbidden value)?  outboundForbidden,TResult Function( LiterLlmError_IdempotencyConflict value)?  idempotencyConflict,TResult Function( LiterLlmError_IdempotencyInFlight value)?  idempotencyInFlight,required TResult orElse(),}){
final _that = this;
switch (_that) {
case LiterLlmError_Authentication() when authentication != null:
return authentication(_that);case LiterLlmError_RateLimited() when rateLimited != null:
return rateLimited(_that);case LiterLlmError_BadRequest() when badRequest != null:
return badRequest(_that);case LiterLlmError_ContextWindowExceeded() when contextWindowExceeded != null:
return contextWindowExceeded(_that);case LiterLlmError_ContentPolicy() when contentPolicy != null:
return contentPolicy(_that);case LiterLlmError_NotFound() when notFound != null:
return notFound(_that);case LiterLlmError_ServerError() when serverError != null:
return serverError(_that);case LiterLlmError_ServiceUnavailable() when serviceUnavailable != null:
return serviceUnavailable(_that);case LiterLlmError_Timeout() when timeout != null:
return timeout(_that);case LiterLlmError_Streaming() when streaming != null:
return streaming(_that);case LiterLlmError_EndpointNotSupported() when endpointNotSupported != null:
return endpointNotSupported(_that);case LiterLlmError_InvalidHeader() when invalidHeader != null:
return invalidHeader(_that);case LiterLlmError_Serialization() when serialization != null:
return serialization(_that);case LiterLlmError_BudgetExceeded() when budgetExceeded != null:
return budgetExceeded(_that);case LiterLlmError_HookRejected() when hookRejected != null:
return hookRejected(_that);case LiterLlmError_InternalError() when internalError != null:
return internalError(_that);case LiterLlmError_OutboundForbidden() when outboundForbidden != null:
return outboundForbidden(_that);case LiterLlmError_IdempotencyConflict() when idempotencyConflict != null:
return idempotencyConflict(_that);case LiterLlmError_IdempotencyInFlight() when idempotencyInFlight != null:
return idempotencyInFlight(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( LiterLlmError_Authentication value)  authentication,required TResult Function( LiterLlmError_RateLimited value)  rateLimited,required TResult Function( LiterLlmError_BadRequest value)  badRequest,required TResult Function( LiterLlmError_ContextWindowExceeded value)  contextWindowExceeded,required TResult Function( LiterLlmError_ContentPolicy value)  contentPolicy,required TResult Function( LiterLlmError_NotFound value)  notFound,required TResult Function( LiterLlmError_ServerError value)  serverError,required TResult Function( LiterLlmError_ServiceUnavailable value)  serviceUnavailable,required TResult Function( LiterLlmError_Timeout value)  timeout,required TResult Function( LiterLlmError_Streaming value)  streaming,required TResult Function( LiterLlmError_EndpointNotSupported value)  endpointNotSupported,required TResult Function( LiterLlmError_InvalidHeader value)  invalidHeader,required TResult Function( LiterLlmError_Serialization value)  serialization,required TResult Function( LiterLlmError_BudgetExceeded value)  budgetExceeded,required TResult Function( LiterLlmError_HookRejected value)  hookRejected,required TResult Function( LiterLlmError_InternalError value)  internalError,required TResult Function( LiterLlmError_OutboundForbidden value)  outboundForbidden,required TResult Function( LiterLlmError_IdempotencyConflict value)  idempotencyConflict,required TResult Function( LiterLlmError_IdempotencyInFlight value)  idempotencyInFlight,}){
final _that = this;
switch (_that) {
case LiterLlmError_Authentication():
return authentication(_that);case LiterLlmError_RateLimited():
return rateLimited(_that);case LiterLlmError_BadRequest():
return badRequest(_that);case LiterLlmError_ContextWindowExceeded():
return contextWindowExceeded(_that);case LiterLlmError_ContentPolicy():
return contentPolicy(_that);case LiterLlmError_NotFound():
return notFound(_that);case LiterLlmError_ServerError():
return serverError(_that);case LiterLlmError_ServiceUnavailable():
return serviceUnavailable(_that);case LiterLlmError_Timeout():
return timeout(_that);case LiterLlmError_Streaming():
return streaming(_that);case LiterLlmError_EndpointNotSupported():
return endpointNotSupported(_that);case LiterLlmError_InvalidHeader():
return invalidHeader(_that);case LiterLlmError_Serialization():
return serialization(_that);case LiterLlmError_BudgetExceeded():
return budgetExceeded(_that);case LiterLlmError_HookRejected():
return hookRejected(_that);case LiterLlmError_InternalError():
return internalError(_that);case LiterLlmError_OutboundForbidden():
return outboundForbidden(_that);case LiterLlmError_IdempotencyConflict():
return idempotencyConflict(_that);case LiterLlmError_IdempotencyInFlight():
return idempotencyInFlight(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( LiterLlmError_Authentication value)?  authentication,TResult? Function( LiterLlmError_RateLimited value)?  rateLimited,TResult? Function( LiterLlmError_BadRequest value)?  badRequest,TResult? Function( LiterLlmError_ContextWindowExceeded value)?  contextWindowExceeded,TResult? Function( LiterLlmError_ContentPolicy value)?  contentPolicy,TResult? Function( LiterLlmError_NotFound value)?  notFound,TResult? Function( LiterLlmError_ServerError value)?  serverError,TResult? Function( LiterLlmError_ServiceUnavailable value)?  serviceUnavailable,TResult? Function( LiterLlmError_Timeout value)?  timeout,TResult? Function( LiterLlmError_Streaming value)?  streaming,TResult? Function( LiterLlmError_EndpointNotSupported value)?  endpointNotSupported,TResult? Function( LiterLlmError_InvalidHeader value)?  invalidHeader,TResult? Function( LiterLlmError_Serialization value)?  serialization,TResult? Function( LiterLlmError_BudgetExceeded value)?  budgetExceeded,TResult? Function( LiterLlmError_HookRejected value)?  hookRejected,TResult? Function( LiterLlmError_InternalError value)?  internalError,TResult? Function( LiterLlmError_OutboundForbidden value)?  outboundForbidden,TResult? Function( LiterLlmError_IdempotencyConflict value)?  idempotencyConflict,TResult? Function( LiterLlmError_IdempotencyInFlight value)?  idempotencyInFlight,}){
final _that = this;
switch (_that) {
case LiterLlmError_Authentication() when authentication != null:
return authentication(_that);case LiterLlmError_RateLimited() when rateLimited != null:
return rateLimited(_that);case LiterLlmError_BadRequest() when badRequest != null:
return badRequest(_that);case LiterLlmError_ContextWindowExceeded() when contextWindowExceeded != null:
return contextWindowExceeded(_that);case LiterLlmError_ContentPolicy() when contentPolicy != null:
return contentPolicy(_that);case LiterLlmError_NotFound() when notFound != null:
return notFound(_that);case LiterLlmError_ServerError() when serverError != null:
return serverError(_that);case LiterLlmError_ServiceUnavailable() when serviceUnavailable != null:
return serviceUnavailable(_that);case LiterLlmError_Timeout() when timeout != null:
return timeout(_that);case LiterLlmError_Streaming() when streaming != null:
return streaming(_that);case LiterLlmError_EndpointNotSupported() when endpointNotSupported != null:
return endpointNotSupported(_that);case LiterLlmError_InvalidHeader() when invalidHeader != null:
return invalidHeader(_that);case LiterLlmError_Serialization() when serialization != null:
return serialization(_that);case LiterLlmError_BudgetExceeded() when budgetExceeded != null:
return budgetExceeded(_that);case LiterLlmError_HookRejected() when hookRejected != null:
return hookRejected(_that);case LiterLlmError_InternalError() when internalError != null:
return internalError(_that);case LiterLlmError_OutboundForbidden() when outboundForbidden != null:
return outboundForbidden(_that);case LiterLlmError_IdempotencyConflict() when idempotencyConflict != null:
return idempotencyConflict(_that);case LiterLlmError_IdempotencyInFlight() when idempotencyInFlight != null:
return idempotencyInFlight(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String message,  PlatformInt64 status)?  authentication,TResult Function( String message,  PlatformInt64 retryAfter)?  rateLimited,TResult Function( String message,  PlatformInt64 status)?  badRequest,TResult Function( String message)?  contextWindowExceeded,TResult Function( String message)?  contentPolicy,TResult Function( String message)?  notFound,TResult Function( String message,  PlatformInt64 status)?  serverError,TResult Function( String message,  PlatformInt64 status)?  serviceUnavailable,TResult Function()?  timeout,TResult Function( String message)?  streaming,TResult Function( String endpoint,  String provider)?  endpointNotSupported,TResult Function( String name,  String reason)?  invalidHeader,TResult Function( String field0)?  serialization,TResult Function( String message,  String model)?  budgetExceeded,TResult Function( String message)?  hookRejected,TResult Function( String message)?  internalError,TResult Function( String url,  String reason)?  outboundForbidden,TResult Function( String key)?  idempotencyConflict,TResult Function( String key)?  idempotencyInFlight,required TResult orElse(),}) {final _that = this;
switch (_that) {
case LiterLlmError_Authentication() when authentication != null:
return authentication(_that.message,_that.status);case LiterLlmError_RateLimited() when rateLimited != null:
return rateLimited(_that.message,_that.retryAfter);case LiterLlmError_BadRequest() when badRequest != null:
return badRequest(_that.message,_that.status);case LiterLlmError_ContextWindowExceeded() when contextWindowExceeded != null:
return contextWindowExceeded(_that.message);case LiterLlmError_ContentPolicy() when contentPolicy != null:
return contentPolicy(_that.message);case LiterLlmError_NotFound() when notFound != null:
return notFound(_that.message);case LiterLlmError_ServerError() when serverError != null:
return serverError(_that.message,_that.status);case LiterLlmError_ServiceUnavailable() when serviceUnavailable != null:
return serviceUnavailable(_that.message,_that.status);case LiterLlmError_Timeout() when timeout != null:
return timeout();case LiterLlmError_Streaming() when streaming != null:
return streaming(_that.message);case LiterLlmError_EndpointNotSupported() when endpointNotSupported != null:
return endpointNotSupported(_that.endpoint,_that.provider);case LiterLlmError_InvalidHeader() when invalidHeader != null:
return invalidHeader(_that.name,_that.reason);case LiterLlmError_Serialization() when serialization != null:
return serialization(_that.field0);case LiterLlmError_BudgetExceeded() when budgetExceeded != null:
return budgetExceeded(_that.message,_that.model);case LiterLlmError_HookRejected() when hookRejected != null:
return hookRejected(_that.message);case LiterLlmError_InternalError() when internalError != null:
return internalError(_that.message);case LiterLlmError_OutboundForbidden() when outboundForbidden != null:
return outboundForbidden(_that.url,_that.reason);case LiterLlmError_IdempotencyConflict() when idempotencyConflict != null:
return idempotencyConflict(_that.key);case LiterLlmError_IdempotencyInFlight() when idempotencyInFlight != null:
return idempotencyInFlight(_that.key);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String message,  PlatformInt64 status)  authentication,required TResult Function( String message,  PlatformInt64 retryAfter)  rateLimited,required TResult Function( String message,  PlatformInt64 status)  badRequest,required TResult Function( String message)  contextWindowExceeded,required TResult Function( String message)  contentPolicy,required TResult Function( String message)  notFound,required TResult Function( String message,  PlatformInt64 status)  serverError,required TResult Function( String message,  PlatformInt64 status)  serviceUnavailable,required TResult Function()  timeout,required TResult Function( String message)  streaming,required TResult Function( String endpoint,  String provider)  endpointNotSupported,required TResult Function( String name,  String reason)  invalidHeader,required TResult Function( String field0)  serialization,required TResult Function( String message,  String model)  budgetExceeded,required TResult Function( String message)  hookRejected,required TResult Function( String message)  internalError,required TResult Function( String url,  String reason)  outboundForbidden,required TResult Function( String key)  idempotencyConflict,required TResult Function( String key)  idempotencyInFlight,}) {final _that = this;
switch (_that) {
case LiterLlmError_Authentication():
return authentication(_that.message,_that.status);case LiterLlmError_RateLimited():
return rateLimited(_that.message,_that.retryAfter);case LiterLlmError_BadRequest():
return badRequest(_that.message,_that.status);case LiterLlmError_ContextWindowExceeded():
return contextWindowExceeded(_that.message);case LiterLlmError_ContentPolicy():
return contentPolicy(_that.message);case LiterLlmError_NotFound():
return notFound(_that.message);case LiterLlmError_ServerError():
return serverError(_that.message,_that.status);case LiterLlmError_ServiceUnavailable():
return serviceUnavailable(_that.message,_that.status);case LiterLlmError_Timeout():
return timeout();case LiterLlmError_Streaming():
return streaming(_that.message);case LiterLlmError_EndpointNotSupported():
return endpointNotSupported(_that.endpoint,_that.provider);case LiterLlmError_InvalidHeader():
return invalidHeader(_that.name,_that.reason);case LiterLlmError_Serialization():
return serialization(_that.field0);case LiterLlmError_BudgetExceeded():
return budgetExceeded(_that.message,_that.model);case LiterLlmError_HookRejected():
return hookRejected(_that.message);case LiterLlmError_InternalError():
return internalError(_that.message);case LiterLlmError_OutboundForbidden():
return outboundForbidden(_that.url,_that.reason);case LiterLlmError_IdempotencyConflict():
return idempotencyConflict(_that.key);case LiterLlmError_IdempotencyInFlight():
return idempotencyInFlight(_that.key);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String message,  PlatformInt64 status)?  authentication,TResult? Function( String message,  PlatformInt64 retryAfter)?  rateLimited,TResult? Function( String message,  PlatformInt64 status)?  badRequest,TResult? Function( String message)?  contextWindowExceeded,TResult? Function( String message)?  contentPolicy,TResult? Function( String message)?  notFound,TResult? Function( String message,  PlatformInt64 status)?  serverError,TResult? Function( String message,  PlatformInt64 status)?  serviceUnavailable,TResult? Function()?  timeout,TResult? Function( String message)?  streaming,TResult? Function( String endpoint,  String provider)?  endpointNotSupported,TResult? Function( String name,  String reason)?  invalidHeader,TResult? Function( String field0)?  serialization,TResult? Function( String message,  String model)?  budgetExceeded,TResult? Function( String message)?  hookRejected,TResult? Function( String message)?  internalError,TResult? Function( String url,  String reason)?  outboundForbidden,TResult? Function( String key)?  idempotencyConflict,TResult? Function( String key)?  idempotencyInFlight,}) {final _that = this;
switch (_that) {
case LiterLlmError_Authentication() when authentication != null:
return authentication(_that.message,_that.status);case LiterLlmError_RateLimited() when rateLimited != null:
return rateLimited(_that.message,_that.retryAfter);case LiterLlmError_BadRequest() when badRequest != null:
return badRequest(_that.message,_that.status);case LiterLlmError_ContextWindowExceeded() when contextWindowExceeded != null:
return contextWindowExceeded(_that.message);case LiterLlmError_ContentPolicy() when contentPolicy != null:
return contentPolicy(_that.message);case LiterLlmError_NotFound() when notFound != null:
return notFound(_that.message);case LiterLlmError_ServerError() when serverError != null:
return serverError(_that.message,_that.status);case LiterLlmError_ServiceUnavailable() when serviceUnavailable != null:
return serviceUnavailable(_that.message,_that.status);case LiterLlmError_Timeout() when timeout != null:
return timeout();case LiterLlmError_Streaming() when streaming != null:
return streaming(_that.message);case LiterLlmError_EndpointNotSupported() when endpointNotSupported != null:
return endpointNotSupported(_that.endpoint,_that.provider);case LiterLlmError_InvalidHeader() when invalidHeader != null:
return invalidHeader(_that.name,_that.reason);case LiterLlmError_Serialization() when serialization != null:
return serialization(_that.field0);case LiterLlmError_BudgetExceeded() when budgetExceeded != null:
return budgetExceeded(_that.message,_that.model);case LiterLlmError_HookRejected() when hookRejected != null:
return hookRejected(_that.message);case LiterLlmError_InternalError() when internalError != null:
return internalError(_that.message);case LiterLlmError_OutboundForbidden() when outboundForbidden != null:
return outboundForbidden(_that.url,_that.reason);case LiterLlmError_IdempotencyConflict() when idempotencyConflict != null:
return idempotencyConflict(_that.key);case LiterLlmError_IdempotencyInFlight() when idempotencyInFlight != null:
return idempotencyInFlight(_that.key);case _:
  return null;

}
}

}

/// @nodoc


class LiterLlmError_Authentication extends LiterLlmError {
  const LiterLlmError_Authentication({required this.message, required this.status}): super._();
  

 final  String message;
 final  PlatformInt64 status;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_AuthenticationCopyWith<LiterLlmError_Authentication> get copyWith => _$LiterLlmError_AuthenticationCopyWithImpl<LiterLlmError_Authentication>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_Authentication&&(identical(other.message, message) || other.message == message)&&(identical(other.status, status) || other.status == status));
}


@override
int get hashCode => Object.hash(runtimeType,message,status);

@override
String toString() {
  return 'LiterLlmError.authentication(message: $message, status: $status)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_AuthenticationCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_AuthenticationCopyWith(LiterLlmError_Authentication value, $Res Function(LiterLlmError_Authentication) _then) = _$LiterLlmError_AuthenticationCopyWithImpl;
@useResult
$Res call({
 String message, PlatformInt64 status
});




}
/// @nodoc
class _$LiterLlmError_AuthenticationCopyWithImpl<$Res>
    implements $LiterLlmError_AuthenticationCopyWith<$Res> {
  _$LiterLlmError_AuthenticationCopyWithImpl(this._self, this._then);

  final LiterLlmError_Authentication _self;
  final $Res Function(LiterLlmError_Authentication) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? status = null,}) {
  return _then(LiterLlmError_Authentication(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class LiterLlmError_RateLimited extends LiterLlmError {
  const LiterLlmError_RateLimited({required this.message, required this.retryAfter}): super._();
  

 final  String message;
 final  PlatformInt64 retryAfter;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_RateLimitedCopyWith<LiterLlmError_RateLimited> get copyWith => _$LiterLlmError_RateLimitedCopyWithImpl<LiterLlmError_RateLimited>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_RateLimited&&(identical(other.message, message) || other.message == message)&&(identical(other.retryAfter, retryAfter) || other.retryAfter == retryAfter));
}


@override
int get hashCode => Object.hash(runtimeType,message,retryAfter);

@override
String toString() {
  return 'LiterLlmError.rateLimited(message: $message, retryAfter: $retryAfter)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_RateLimitedCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_RateLimitedCopyWith(LiterLlmError_RateLimited value, $Res Function(LiterLlmError_RateLimited) _then) = _$LiterLlmError_RateLimitedCopyWithImpl;
@useResult
$Res call({
 String message, PlatformInt64 retryAfter
});




}
/// @nodoc
class _$LiterLlmError_RateLimitedCopyWithImpl<$Res>
    implements $LiterLlmError_RateLimitedCopyWith<$Res> {
  _$LiterLlmError_RateLimitedCopyWithImpl(this._self, this._then);

  final LiterLlmError_RateLimited _self;
  final $Res Function(LiterLlmError_RateLimited) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? retryAfter = null,}) {
  return _then(LiterLlmError_RateLimited(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,retryAfter: null == retryAfter ? _self.retryAfter : retryAfter // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class LiterLlmError_BadRequest extends LiterLlmError {
  const LiterLlmError_BadRequest({required this.message, required this.status}): super._();
  

 final  String message;
 final  PlatformInt64 status;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_BadRequestCopyWith<LiterLlmError_BadRequest> get copyWith => _$LiterLlmError_BadRequestCopyWithImpl<LiterLlmError_BadRequest>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_BadRequest&&(identical(other.message, message) || other.message == message)&&(identical(other.status, status) || other.status == status));
}


@override
int get hashCode => Object.hash(runtimeType,message,status);

@override
String toString() {
  return 'LiterLlmError.badRequest(message: $message, status: $status)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_BadRequestCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_BadRequestCopyWith(LiterLlmError_BadRequest value, $Res Function(LiterLlmError_BadRequest) _then) = _$LiterLlmError_BadRequestCopyWithImpl;
@useResult
$Res call({
 String message, PlatformInt64 status
});




}
/// @nodoc
class _$LiterLlmError_BadRequestCopyWithImpl<$Res>
    implements $LiterLlmError_BadRequestCopyWith<$Res> {
  _$LiterLlmError_BadRequestCopyWithImpl(this._self, this._then);

  final LiterLlmError_BadRequest _self;
  final $Res Function(LiterLlmError_BadRequest) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? status = null,}) {
  return _then(LiterLlmError_BadRequest(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class LiterLlmError_ContextWindowExceeded extends LiterLlmError {
  const LiterLlmError_ContextWindowExceeded({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_ContextWindowExceededCopyWith<LiterLlmError_ContextWindowExceeded> get copyWith => _$LiterLlmError_ContextWindowExceededCopyWithImpl<LiterLlmError_ContextWindowExceeded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_ContextWindowExceeded&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.contextWindowExceeded(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_ContextWindowExceededCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_ContextWindowExceededCopyWith(LiterLlmError_ContextWindowExceeded value, $Res Function(LiterLlmError_ContextWindowExceeded) _then) = _$LiterLlmError_ContextWindowExceededCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_ContextWindowExceededCopyWithImpl<$Res>
    implements $LiterLlmError_ContextWindowExceededCopyWith<$Res> {
  _$LiterLlmError_ContextWindowExceededCopyWithImpl(this._self, this._then);

  final LiterLlmError_ContextWindowExceeded _self;
  final $Res Function(LiterLlmError_ContextWindowExceeded) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_ContextWindowExceeded(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_ContentPolicy extends LiterLlmError {
  const LiterLlmError_ContentPolicy({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_ContentPolicyCopyWith<LiterLlmError_ContentPolicy> get copyWith => _$LiterLlmError_ContentPolicyCopyWithImpl<LiterLlmError_ContentPolicy>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_ContentPolicy&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.contentPolicy(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_ContentPolicyCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_ContentPolicyCopyWith(LiterLlmError_ContentPolicy value, $Res Function(LiterLlmError_ContentPolicy) _then) = _$LiterLlmError_ContentPolicyCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_ContentPolicyCopyWithImpl<$Res>
    implements $LiterLlmError_ContentPolicyCopyWith<$Res> {
  _$LiterLlmError_ContentPolicyCopyWithImpl(this._self, this._then);

  final LiterLlmError_ContentPolicy _self;
  final $Res Function(LiterLlmError_ContentPolicy) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_ContentPolicy(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_NotFound extends LiterLlmError {
  const LiterLlmError_NotFound({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_NotFoundCopyWith<LiterLlmError_NotFound> get copyWith => _$LiterLlmError_NotFoundCopyWithImpl<LiterLlmError_NotFound>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_NotFound&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.notFound(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_NotFoundCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_NotFoundCopyWith(LiterLlmError_NotFound value, $Res Function(LiterLlmError_NotFound) _then) = _$LiterLlmError_NotFoundCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_NotFoundCopyWithImpl<$Res>
    implements $LiterLlmError_NotFoundCopyWith<$Res> {
  _$LiterLlmError_NotFoundCopyWithImpl(this._self, this._then);

  final LiterLlmError_NotFound _self;
  final $Res Function(LiterLlmError_NotFound) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_NotFound(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_ServerError extends LiterLlmError {
  const LiterLlmError_ServerError({required this.message, required this.status}): super._();
  

 final  String message;
 final  PlatformInt64 status;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_ServerErrorCopyWith<LiterLlmError_ServerError> get copyWith => _$LiterLlmError_ServerErrorCopyWithImpl<LiterLlmError_ServerError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_ServerError&&(identical(other.message, message) || other.message == message)&&(identical(other.status, status) || other.status == status));
}


@override
int get hashCode => Object.hash(runtimeType,message,status);

@override
String toString() {
  return 'LiterLlmError.serverError(message: $message, status: $status)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_ServerErrorCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_ServerErrorCopyWith(LiterLlmError_ServerError value, $Res Function(LiterLlmError_ServerError) _then) = _$LiterLlmError_ServerErrorCopyWithImpl;
@useResult
$Res call({
 String message, PlatformInt64 status
});




}
/// @nodoc
class _$LiterLlmError_ServerErrorCopyWithImpl<$Res>
    implements $LiterLlmError_ServerErrorCopyWith<$Res> {
  _$LiterLlmError_ServerErrorCopyWithImpl(this._self, this._then);

  final LiterLlmError_ServerError _self;
  final $Res Function(LiterLlmError_ServerError) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? status = null,}) {
  return _then(LiterLlmError_ServerError(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class LiterLlmError_ServiceUnavailable extends LiterLlmError {
  const LiterLlmError_ServiceUnavailable({required this.message, required this.status}): super._();
  

 final  String message;
 final  PlatformInt64 status;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_ServiceUnavailableCopyWith<LiterLlmError_ServiceUnavailable> get copyWith => _$LiterLlmError_ServiceUnavailableCopyWithImpl<LiterLlmError_ServiceUnavailable>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_ServiceUnavailable&&(identical(other.message, message) || other.message == message)&&(identical(other.status, status) || other.status == status));
}


@override
int get hashCode => Object.hash(runtimeType,message,status);

@override
String toString() {
  return 'LiterLlmError.serviceUnavailable(message: $message, status: $status)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_ServiceUnavailableCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_ServiceUnavailableCopyWith(LiterLlmError_ServiceUnavailable value, $Res Function(LiterLlmError_ServiceUnavailable) _then) = _$LiterLlmError_ServiceUnavailableCopyWithImpl;
@useResult
$Res call({
 String message, PlatformInt64 status
});




}
/// @nodoc
class _$LiterLlmError_ServiceUnavailableCopyWithImpl<$Res>
    implements $LiterLlmError_ServiceUnavailableCopyWith<$Res> {
  _$LiterLlmError_ServiceUnavailableCopyWithImpl(this._self, this._then);

  final LiterLlmError_ServiceUnavailable _self;
  final $Res Function(LiterLlmError_ServiceUnavailable) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? status = null,}) {
  return _then(LiterLlmError_ServiceUnavailable(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class LiterLlmError_Timeout extends LiterLlmError {
  const LiterLlmError_Timeout(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_Timeout);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'LiterLlmError.timeout()';
}


}




/// @nodoc


class LiterLlmError_Streaming extends LiterLlmError {
  const LiterLlmError_Streaming({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_StreamingCopyWith<LiterLlmError_Streaming> get copyWith => _$LiterLlmError_StreamingCopyWithImpl<LiterLlmError_Streaming>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_Streaming&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.streaming(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_StreamingCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_StreamingCopyWith(LiterLlmError_Streaming value, $Res Function(LiterLlmError_Streaming) _then) = _$LiterLlmError_StreamingCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_StreamingCopyWithImpl<$Res>
    implements $LiterLlmError_StreamingCopyWith<$Res> {
  _$LiterLlmError_StreamingCopyWithImpl(this._self, this._then);

  final LiterLlmError_Streaming _self;
  final $Res Function(LiterLlmError_Streaming) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_Streaming(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_EndpointNotSupported extends LiterLlmError {
  const LiterLlmError_EndpointNotSupported({required this.endpoint, required this.provider}): super._();
  

 final  String endpoint;
 final  String provider;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_EndpointNotSupportedCopyWith<LiterLlmError_EndpointNotSupported> get copyWith => _$LiterLlmError_EndpointNotSupportedCopyWithImpl<LiterLlmError_EndpointNotSupported>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_EndpointNotSupported&&(identical(other.endpoint, endpoint) || other.endpoint == endpoint)&&(identical(other.provider, provider) || other.provider == provider));
}


@override
int get hashCode => Object.hash(runtimeType,endpoint,provider);

@override
String toString() {
  return 'LiterLlmError.endpointNotSupported(endpoint: $endpoint, provider: $provider)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_EndpointNotSupportedCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_EndpointNotSupportedCopyWith(LiterLlmError_EndpointNotSupported value, $Res Function(LiterLlmError_EndpointNotSupported) _then) = _$LiterLlmError_EndpointNotSupportedCopyWithImpl;
@useResult
$Res call({
 String endpoint, String provider
});




}
/// @nodoc
class _$LiterLlmError_EndpointNotSupportedCopyWithImpl<$Res>
    implements $LiterLlmError_EndpointNotSupportedCopyWith<$Res> {
  _$LiterLlmError_EndpointNotSupportedCopyWithImpl(this._self, this._then);

  final LiterLlmError_EndpointNotSupported _self;
  final $Res Function(LiterLlmError_EndpointNotSupported) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? endpoint = null,Object? provider = null,}) {
  return _then(LiterLlmError_EndpointNotSupported(
endpoint: null == endpoint ? _self.endpoint : endpoint // ignore: cast_nullable_to_non_nullable
as String,provider: null == provider ? _self.provider : provider // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_InvalidHeader extends LiterLlmError {
  const LiterLlmError_InvalidHeader({required this.name, required this.reason}): super._();
  

 final  String name;
 final  String reason;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_InvalidHeaderCopyWith<LiterLlmError_InvalidHeader> get copyWith => _$LiterLlmError_InvalidHeaderCopyWithImpl<LiterLlmError_InvalidHeader>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_InvalidHeader&&(identical(other.name, name) || other.name == name)&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,name,reason);

@override
String toString() {
  return 'LiterLlmError.invalidHeader(name: $name, reason: $reason)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_InvalidHeaderCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_InvalidHeaderCopyWith(LiterLlmError_InvalidHeader value, $Res Function(LiterLlmError_InvalidHeader) _then) = _$LiterLlmError_InvalidHeaderCopyWithImpl;
@useResult
$Res call({
 String name, String reason
});




}
/// @nodoc
class _$LiterLlmError_InvalidHeaderCopyWithImpl<$Res>
    implements $LiterLlmError_InvalidHeaderCopyWith<$Res> {
  _$LiterLlmError_InvalidHeaderCopyWithImpl(this._self, this._then);

  final LiterLlmError_InvalidHeader _self;
  final $Res Function(LiterLlmError_InvalidHeader) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,Object? reason = null,}) {
  return _then(LiterLlmError_InvalidHeader(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_Serialization extends LiterLlmError {
  const LiterLlmError_Serialization({required this.field0}): super._();
  

 final  String field0;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_SerializationCopyWith<LiterLlmError_Serialization> get copyWith => _$LiterLlmError_SerializationCopyWithImpl<LiterLlmError_Serialization>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_Serialization&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'LiterLlmError.serialization(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_SerializationCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_SerializationCopyWith(LiterLlmError_Serialization value, $Res Function(LiterLlmError_Serialization) _then) = _$LiterLlmError_SerializationCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$LiterLlmError_SerializationCopyWithImpl<$Res>
    implements $LiterLlmError_SerializationCopyWith<$Res> {
  _$LiterLlmError_SerializationCopyWithImpl(this._self, this._then);

  final LiterLlmError_Serialization _self;
  final $Res Function(LiterLlmError_Serialization) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(LiterLlmError_Serialization(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_BudgetExceeded extends LiterLlmError {
  const LiterLlmError_BudgetExceeded({required this.message, required this.model}): super._();
  

 final  String message;
 final  String model;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_BudgetExceededCopyWith<LiterLlmError_BudgetExceeded> get copyWith => _$LiterLlmError_BudgetExceededCopyWithImpl<LiterLlmError_BudgetExceeded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_BudgetExceeded&&(identical(other.message, message) || other.message == message)&&(identical(other.model, model) || other.model == model));
}


@override
int get hashCode => Object.hash(runtimeType,message,model);

@override
String toString() {
  return 'LiterLlmError.budgetExceeded(message: $message, model: $model)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_BudgetExceededCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_BudgetExceededCopyWith(LiterLlmError_BudgetExceeded value, $Res Function(LiterLlmError_BudgetExceeded) _then) = _$LiterLlmError_BudgetExceededCopyWithImpl;
@useResult
$Res call({
 String message, String model
});




}
/// @nodoc
class _$LiterLlmError_BudgetExceededCopyWithImpl<$Res>
    implements $LiterLlmError_BudgetExceededCopyWith<$Res> {
  _$LiterLlmError_BudgetExceededCopyWithImpl(this._self, this._then);

  final LiterLlmError_BudgetExceeded _self;
  final $Res Function(LiterLlmError_BudgetExceeded) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,Object? model = null,}) {
  return _then(LiterLlmError_BudgetExceeded(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,model: null == model ? _self.model : model // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_HookRejected extends LiterLlmError {
  const LiterLlmError_HookRejected({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_HookRejectedCopyWith<LiterLlmError_HookRejected> get copyWith => _$LiterLlmError_HookRejectedCopyWithImpl<LiterLlmError_HookRejected>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_HookRejected&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.hookRejected(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_HookRejectedCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_HookRejectedCopyWith(LiterLlmError_HookRejected value, $Res Function(LiterLlmError_HookRejected) _then) = _$LiterLlmError_HookRejectedCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_HookRejectedCopyWithImpl<$Res>
    implements $LiterLlmError_HookRejectedCopyWith<$Res> {
  _$LiterLlmError_HookRejectedCopyWithImpl(this._self, this._then);

  final LiterLlmError_HookRejected _self;
  final $Res Function(LiterLlmError_HookRejected) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_HookRejected(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_InternalError extends LiterLlmError {
  const LiterLlmError_InternalError({required this.message}): super._();
  

 final  String message;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_InternalErrorCopyWith<LiterLlmError_InternalError> get copyWith => _$LiterLlmError_InternalErrorCopyWithImpl<LiterLlmError_InternalError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_InternalError&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,message);

@override
String toString() {
  return 'LiterLlmError.internalError(message: $message)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_InternalErrorCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_InternalErrorCopyWith(LiterLlmError_InternalError value, $Res Function(LiterLlmError_InternalError) _then) = _$LiterLlmError_InternalErrorCopyWithImpl;
@useResult
$Res call({
 String message
});




}
/// @nodoc
class _$LiterLlmError_InternalErrorCopyWithImpl<$Res>
    implements $LiterLlmError_InternalErrorCopyWith<$Res> {
  _$LiterLlmError_InternalErrorCopyWithImpl(this._self, this._then);

  final LiterLlmError_InternalError _self;
  final $Res Function(LiterLlmError_InternalError) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? message = null,}) {
  return _then(LiterLlmError_InternalError(
message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_OutboundForbidden extends LiterLlmError {
  const LiterLlmError_OutboundForbidden({required this.url, required this.reason}): super._();
  

 final  String url;
 final  String reason;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_OutboundForbiddenCopyWith<LiterLlmError_OutboundForbidden> get copyWith => _$LiterLlmError_OutboundForbiddenCopyWithImpl<LiterLlmError_OutboundForbidden>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_OutboundForbidden&&(identical(other.url, url) || other.url == url)&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,url,reason);

@override
String toString() {
  return 'LiterLlmError.outboundForbidden(url: $url, reason: $reason)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_OutboundForbiddenCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_OutboundForbiddenCopyWith(LiterLlmError_OutboundForbidden value, $Res Function(LiterLlmError_OutboundForbidden) _then) = _$LiterLlmError_OutboundForbiddenCopyWithImpl;
@useResult
$Res call({
 String url, String reason
});




}
/// @nodoc
class _$LiterLlmError_OutboundForbiddenCopyWithImpl<$Res>
    implements $LiterLlmError_OutboundForbiddenCopyWith<$Res> {
  _$LiterLlmError_OutboundForbiddenCopyWithImpl(this._self, this._then);

  final LiterLlmError_OutboundForbidden _self;
  final $Res Function(LiterLlmError_OutboundForbidden) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,Object? reason = null,}) {
  return _then(LiterLlmError_OutboundForbidden(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_IdempotencyConflict extends LiterLlmError {
  const LiterLlmError_IdempotencyConflict({required this.key}): super._();
  

 final  String key;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_IdempotencyConflictCopyWith<LiterLlmError_IdempotencyConflict> get copyWith => _$LiterLlmError_IdempotencyConflictCopyWithImpl<LiterLlmError_IdempotencyConflict>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_IdempotencyConflict&&(identical(other.key, key) || other.key == key));
}


@override
int get hashCode => Object.hash(runtimeType,key);

@override
String toString() {
  return 'LiterLlmError.idempotencyConflict(key: $key)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_IdempotencyConflictCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_IdempotencyConflictCopyWith(LiterLlmError_IdempotencyConflict value, $Res Function(LiterLlmError_IdempotencyConflict) _then) = _$LiterLlmError_IdempotencyConflictCopyWithImpl;
@useResult
$Res call({
 String key
});




}
/// @nodoc
class _$LiterLlmError_IdempotencyConflictCopyWithImpl<$Res>
    implements $LiterLlmError_IdempotencyConflictCopyWith<$Res> {
  _$LiterLlmError_IdempotencyConflictCopyWithImpl(this._self, this._then);

  final LiterLlmError_IdempotencyConflict _self;
  final $Res Function(LiterLlmError_IdempotencyConflict) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? key = null,}) {
  return _then(LiterLlmError_IdempotencyConflict(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LiterLlmError_IdempotencyInFlight extends LiterLlmError {
  const LiterLlmError_IdempotencyInFlight({required this.key}): super._();
  

 final  String key;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LiterLlmError_IdempotencyInFlightCopyWith<LiterLlmError_IdempotencyInFlight> get copyWith => _$LiterLlmError_IdempotencyInFlightCopyWithImpl<LiterLlmError_IdempotencyInFlight>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is LiterLlmError_IdempotencyInFlight&&(identical(other.key, key) || other.key == key));
}


@override
int get hashCode => Object.hash(runtimeType,key);

@override
String toString() {
  return 'LiterLlmError.idempotencyInFlight(key: $key)';
}


}

/// @nodoc
abstract mixin class $LiterLlmError_IdempotencyInFlightCopyWith<$Res> implements $LiterLlmErrorCopyWith<$Res> {
  factory $LiterLlmError_IdempotencyInFlightCopyWith(LiterLlmError_IdempotencyInFlight value, $Res Function(LiterLlmError_IdempotencyInFlight) _then) = _$LiterLlmError_IdempotencyInFlightCopyWithImpl;
@useResult
$Res call({
 String key
});




}
/// @nodoc
class _$LiterLlmError_IdempotencyInFlightCopyWithImpl<$Res>
    implements $LiterLlmError_IdempotencyInFlightCopyWith<$Res> {
  _$LiterLlmError_IdempotencyInFlightCopyWithImpl(this._self, this._then);

  final LiterLlmError_IdempotencyInFlight _self;
  final $Res Function(LiterLlmError_IdempotencyInFlight) _then;

/// Create a copy of LiterLlmError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? key = null,}) {
  return _then(LiterLlmError_IdempotencyInFlight(
key: null == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as String,
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
  

/// The document URL (HTTP/HTTPS).
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
/// MIME type (e.g. `"application/pdf"`, `"image/png"`, `"image/jpeg"`).
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
