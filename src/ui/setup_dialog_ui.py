# -*- coding: utf-8 -*-

################################################################################
## Form generated from reading UI file 'setup-dialog.ui'
##
## Created by: Qt User Interface Compiler version 6.6.2
##
## WARNING! All changes made in this file will be lost when recompiling UI file!
################################################################################

try:
    from PySide6.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
        QMetaObject, QObject, QPoint, QRect,
        QSize, QTime, QUrl, Qt)
    from PySide6.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
        QFont, QFontDatabase, QGradient, QIcon,
        QImage, QKeySequence, QLinearGradient, QPainter,
        QPalette, QPixmap, QRadialGradient, QTransform)
    from PySide6.QtWidgets import (QApplication, QComboBox, QDialog, QFormLayout,
        QHBoxLayout, QLabel, QLayout, QLineEdit,
        QPushButton, QSizePolicy, QTabWidget, QWidget)
except ImportError:
    from PyQt5.QtCore import (QCoreApplication, QDate, QDateTime, QLocale,
        QMetaObject, QObject, QPoint, QRect,
        QSize, QTime, QUrl, Qt)
    from PyQt5.QtGui import (QBrush, QColor, QConicalGradient, QCursor,
        QFont, QFontDatabase, QGradient, QIcon,
        QImage, QKeySequence, QLinearGradient, QPainter,
        QPalette, QPixmap, QRadialGradient, QTransform)
    from PyQt5.QtWidgets import (QApplication, QComboBox, QDialog, QFormLayout,
        QHBoxLayout, QLabel, QLayout, QLineEdit,
        QPushButton, QSizePolicy, QTabWidget, QWidget)

from . import uds_rc

class Ui_UdsActorSetupDialog(object):
    def setupUi(self, UdsActorSetupDialog):
        if not UdsActorSetupDialog.objectName():
            UdsActorSetupDialog.setObjectName(u"UdsActorSetupDialog")
        UdsActorSetupDialog.setWindowModality(Qt.WindowModal)
        UdsActorSetupDialog.resize(590, 307)
        sizePolicy = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Preferred)
        sizePolicy.setHorizontalStretch(0)
        sizePolicy.setVerticalStretch(0)
        sizePolicy.setHeightForWidth(UdsActorSetupDialog.sizePolicy().hasHeightForWidth())
        UdsActorSetupDialog.setSizePolicy(sizePolicy)
        font = QFont()
        font.setFamilies([u"Verdana"])
        font.setPointSize(9)
        UdsActorSetupDialog.setFont(font)
        UdsActorSetupDialog.setContextMenuPolicy(Qt.DefaultContextMenu)
        icon = QIcon()
        icon.addFile(u":/img/img/uds-icon.png", QSize(), QIcon.Normal, QIcon.Off)
        UdsActorSetupDialog.setWindowIcon(icon)
        UdsActorSetupDialog.setAutoFillBackground(False)
        UdsActorSetupDialog.setLocale(QLocale(QLocale.English, QLocale.UnitedStates))
        UdsActorSetupDialog.setSizeGripEnabled(False)
        UdsActorSetupDialog.setModal(True)
        self.registerButton = QPushButton(UdsActorSetupDialog)
        self.registerButton.setObjectName(u"registerButton")
        self.registerButton.setEnabled(False)
        self.registerButton.setGeometry(QRect(10, 270, 181, 23))
        self.registerButton.setMinimumSize(QSize(181, 0))
        self.registerButton.setContextMenuPolicy(Qt.DefaultContextMenu)
        self.closeButton = QPushButton(UdsActorSetupDialog)
        self.closeButton.setObjectName(u"closeButton")
        self.closeButton.setGeometry(QRect(410, 270, 171, 23))
        sizePolicy1 = QSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Fixed)
        sizePolicy1.setHorizontalStretch(0)
        sizePolicy1.setVerticalStretch(0)
        sizePolicy1.setHeightForWidth(self.closeButton.sizePolicy().hasHeightForWidth())
        self.closeButton.setSizePolicy(sizePolicy1)
        self.closeButton.setMinimumSize(QSize(171, 0))
        self.tabWidget = QTabWidget(UdsActorSetupDialog)
        self.tabWidget.setObjectName(u"tabWidget")
        self.tabWidget.setGeometry(QRect(10, 10, 571, 241))
        self.tab_uds = QWidget()
        self.tab_uds.setObjectName(u"tab_uds")
        self.layoutWidget = QWidget(self.tab_uds)
        self.layoutWidget.setObjectName(u"layoutWidget")
        self.layoutWidget.setGeometry(QRect(10, 10, 551, 191))
        self.formLayout = QFormLayout(self.layoutWidget)
        self.formLayout.setObjectName(u"formLayout")
        self.formLayout.setSizeConstraint(QLayout.SetDefaultConstraint)
        self.formLayout.setFieldGrowthPolicy(QFormLayout.AllNonFixedFieldsGrow)
        self.formLayout.setVerticalSpacing(16)
        self.formLayout.setContentsMargins(0, 0, 0, 0)
        self.label_host = QLabel(self.layoutWidget)
        self.label_host.setObjectName(u"label_host")

        self.formLayout.setWidget(1, QFormLayout.LabelRole, self.label_host)

        self.host = QLineEdit(self.layoutWidget)
        self.host.setObjectName(u"host")
        self.host.setAcceptDrops(False)

        self.formLayout.setWidget(1, QFormLayout.FieldRole, self.host)

        self.label_auth = QLabel(self.layoutWidget)
        self.label_auth.setObjectName(u"label_auth")

        self.formLayout.setWidget(2, QFormLayout.LabelRole, self.label_auth)

        self.authenticators = QComboBox(self.layoutWidget)
        self.authenticators.setObjectName(u"authenticators")

        self.formLayout.setWidget(2, QFormLayout.FieldRole, self.authenticators)

        self.label_username = QLabel(self.layoutWidget)
        self.label_username.setObjectName(u"label_username")

        self.formLayout.setWidget(3, QFormLayout.LabelRole, self.label_username)

        self.username = QLineEdit(self.layoutWidget)
        self.username.setObjectName(u"username")

        self.formLayout.setWidget(3, QFormLayout.FieldRole, self.username)

        self.label_password = QLabel(self.layoutWidget)
        self.label_password.setObjectName(u"label_password")

        self.formLayout.setWidget(4, QFormLayout.LabelRole, self.label_password)

        self.password = QLineEdit(self.layoutWidget)
        self.password.setObjectName(u"password")
        self.password.setEchoMode(QLineEdit.Password)

        self.formLayout.setWidget(4, QFormLayout.FieldRole, self.password)

        self.validateCertificate = QComboBox(self.layoutWidget)
        self.validateCertificate.addItem("")
        self.validateCertificate.addItem("")
        self.validateCertificate.setObjectName(u"validateCertificate")

        self.formLayout.setWidget(0, QFormLayout.FieldRole, self.validateCertificate)

        self.label_security = QLabel(self.layoutWidget)
        self.label_security.setObjectName(u"label_security")

        self.formLayout.setWidget(0, QFormLayout.LabelRole, self.label_security)

        self.label_host.raise_()
        self.host.raise_()
        self.label_auth.raise_()
        self.label_username.raise_()
        self.username.raise_()
        self.label_password.raise_()
        self.password.raise_()
        self.validateCertificate.raise_()
        self.label_security.raise_()
        self.authenticators.raise_()
        self.tabWidget.addTab(self.tab_uds, "")
        self.tab_advanced = QWidget()
        self.tab_advanced.setObjectName(u"tab_advanced")
        self.layoutWidget_2 = QWidget(self.tab_advanced)
        self.layoutWidget_2.setObjectName(u"layoutWidget_2")
        self.layoutWidget_2.setGeometry(QRect(10, 10, 551, 161))
        self.formLayout_2 = QFormLayout(self.layoutWidget_2)
        self.formLayout_2.setObjectName(u"formLayout_2")
        self.formLayout_2.setFieldGrowthPolicy(QFormLayout.AllNonFixedFieldsGrow)
        self.formLayout_2.setVerticalSpacing(16)
        self.formLayout_2.setContentsMargins(0, 0, 0, 0)
        self.label_host_2 = QLabel(self.layoutWidget_2)
        self.label_host_2.setObjectName(u"label_host_2")

        self.formLayout_2.setWidget(0, QFormLayout.LabelRole, self.label_host_2)

        self.horizontalLayout = QHBoxLayout()
        self.horizontalLayout.setSpacing(4)
        self.horizontalLayout.setObjectName(u"horizontalLayout")
        self.horizontalLayout.setContentsMargins(-1, 0, -1, -1)
        self.preCommand = QLineEdit(self.layoutWidget_2)
        self.preCommand.setObjectName(u"preCommand")
        self.preCommand.setAcceptDrops(False)

        self.horizontalLayout.addWidget(self.preCommand)

        self.browsePreconnectButton = QPushButton(self.layoutWidget_2)
        self.browsePreconnectButton.setObjectName(u"browsePreconnectButton")
        self.browsePreconnectButton.setAutoDefault(False)
        self.browsePreconnectButton.setFlat(False)

        self.horizontalLayout.addWidget(self.browsePreconnectButton)


        self.formLayout_2.setLayout(0, QFormLayout.FieldRole, self.horizontalLayout)

        self.label_username_2 = QLabel(self.layoutWidget_2)
        self.label_username_2.setObjectName(u"label_username_2")

        self.formLayout_2.setWidget(1, QFormLayout.LabelRole, self.label_username_2)

        self.horizontalLayout_2 = QHBoxLayout()
        self.horizontalLayout_2.setSpacing(4)
        self.horizontalLayout_2.setObjectName(u"horizontalLayout_2")
        self.horizontalLayout_2.setContentsMargins(-1, 0, -1, -1)
        self.runonceCommand = QLineEdit(self.layoutWidget_2)
        self.runonceCommand.setObjectName(u"runonceCommand")

        self.horizontalLayout_2.addWidget(self.runonceCommand)

        self.browseRunOnceButton = QPushButton(self.layoutWidget_2)
        self.browseRunOnceButton.setObjectName(u"browseRunOnceButton")
        self.browseRunOnceButton.setAutoDefault(False)

        self.horizontalLayout_2.addWidget(self.browseRunOnceButton)


        self.formLayout_2.setLayout(1, QFormLayout.FieldRole, self.horizontalLayout_2)

        self.label_password_2 = QLabel(self.layoutWidget_2)
        self.label_password_2.setObjectName(u"label_password_2")

        self.formLayout_2.setWidget(2, QFormLayout.LabelRole, self.label_password_2)

        self.horizontalLayout_3 = QHBoxLayout()
        self.horizontalLayout_3.setSpacing(4)
        self.horizontalLayout_3.setObjectName(u"horizontalLayout_3")
        self.horizontalLayout_3.setContentsMargins(-1, 0, -1, -1)
        self.postConfigCommand = QLineEdit(self.layoutWidget_2)
        self.postConfigCommand.setObjectName(u"postConfigCommand")
        self.postConfigCommand.setEchoMode(QLineEdit.Normal)

        self.horizontalLayout_3.addWidget(self.postConfigCommand)

        self.browsePostConfigButton = QPushButton(self.layoutWidget_2)
        self.browsePostConfigButton.setObjectName(u"browsePostConfigButton")
        self.browsePostConfigButton.setAutoDefault(False)

        self.horizontalLayout_3.addWidget(self.browsePostConfigButton)


        self.formLayout_2.setLayout(2, QFormLayout.FieldRole, self.horizontalLayout_3)

        self.label_loglevel = QLabel(self.layoutWidget_2)
        self.label_loglevel.setObjectName(u"label_loglevel")

        self.formLayout_2.setWidget(3, QFormLayout.LabelRole, self.label_loglevel)

        self.logLevelComboBox = QComboBox(self.layoutWidget_2)
        self.logLevelComboBox.addItem(u"DEBUG")
        self.logLevelComboBox.addItem(u"INFO")
        self.logLevelComboBox.addItem(u"ERROR")
        self.logLevelComboBox.addItem(u"FATAL")
        self.logLevelComboBox.setObjectName(u"logLevelComboBox")
        self.logLevelComboBox.setFrame(True)

        self.formLayout_2.setWidget(3, QFormLayout.FieldRole, self.logLevelComboBox)

        self.tabWidget.addTab(self.tab_advanced, "")
        self.testButton = QPushButton(UdsActorSetupDialog)
        self.testButton.setObjectName(u"testButton")
        self.testButton.setEnabled(False)
        self.testButton.setGeometry(QRect(210, 270, 181, 23))
        self.testButton.setMinimumSize(QSize(181, 0))

        self.retranslateUi(UdsActorSetupDialog)
        self.closeButton.clicked.connect(UdsActorSetupDialog.finish)
        self.registerButton.clicked.connect(UdsActorSetupDialog.registerWithUDS)
        self.host.textChanged.connect(UdsActorSetupDialog.textChanged)
        self.username.textChanged.connect(UdsActorSetupDialog.textChanged)
        self.password.textChanged.connect(UdsActorSetupDialog.textChanged)
        self.browsePreconnectButton.clicked.connect(UdsActorSetupDialog.browsePreconnect)
        self.browsePostConfigButton.clicked.connect(UdsActorSetupDialog.browsePostConfig)
        self.browseRunOnceButton.clicked.connect(UdsActorSetupDialog.browseRunOnce)
        self.host.editingFinished.connect(UdsActorSetupDialog.updateAuthenticators)
        self.authenticators.currentTextChanged.connect(UdsActorSetupDialog.textChanged)
        self.testButton.clicked.connect(UdsActorSetupDialog.testUDSServer)

        self.tabWidget.setCurrentIndex(0)
        self.validateCertificate.setCurrentIndex(1)
        self.logLevelComboBox.setCurrentIndex(1)


        QMetaObject.connectSlotsByName(UdsActorSetupDialog)
    # setupUi

    def retranslateUi(self, UdsActorSetupDialog):
        UdsActorSetupDialog.setWindowTitle(QCoreApplication.translate("UdsActorSetupDialog", u"UDS Actor Configuration Tool", None))
#if QT_CONFIG(tooltip)
        self.registerButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Click to register Actor with UDS Broker", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.registerButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Click on this button to register Actor with UDS Broker.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.registerButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Register with UDS", None))
#if QT_CONFIG(tooltip)
        self.closeButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Closes UDS Actor Configuration (discard pending changes if any)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.closeButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Exits the UDS Actor Configuration Tool</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.closeButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Close", None))
        self.label_host.setText(QCoreApplication.translate("UdsActorSetupDialog", u"UDS Server", None))
#if QT_CONFIG(tooltip)
        self.host.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Uds Broker Server Addres. Use IP or FQDN", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.host.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"Enter here the UDS Broker Addres using either its IP address or its FQDN address", None))
#endif // QT_CONFIG(whatsthis)
        self.label_auth.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Authenticator", None))
#if QT_CONFIG(whatsthis)
        self.authenticators.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Select the UDS Broker authenticator for credentials validation</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.label_username.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Username", None))
#if QT_CONFIG(tooltip)
        self.username.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"UDS user with administration rights (Will not be stored on template)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.username.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Administrator user on UDS Server.</p><p>Note: This credential will not be stored on client. Will be used to obtain an unique token for this image.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.label_password.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Password", None))
#if QT_CONFIG(tooltip)
        self.password.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Password for user (Will not be stored on template)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.password.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Administrator password for the user on UDS Server.</p><p>Note: This credential will not be stored on client. Will be used to obtain an unique key for this image.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.validateCertificate.setItemText(0, QCoreApplication.translate("UdsActorSetupDialog", u"Ignore certificate", None))
        self.validateCertificate.setItemText(1, QCoreApplication.translate("UdsActorSetupDialog", u"Verify certificate", None))

#if QT_CONFIG(tooltip)
        self.validateCertificate.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Select communication security with broker", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.validateCertificate.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Select the security for communications with UDS Broker.</p><p>The recommended method of communication is <span style=\" font-weight:600;\">Use SSL</span>, but selection needs to be acording to your broker configuration.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.label_security.setText(QCoreApplication.translate("UdsActorSetupDialog", u"SSL Validation", None))
        self.tabWidget.setTabText(self.tabWidget.indexOf(self.tab_uds), QCoreApplication.translate("UdsActorSetupDialog", u"UDS Server", None))
        self.label_host_2.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Preconnect", None))
#if QT_CONFIG(tooltip)
        self.preCommand.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Pre connection command. Executed just before the user is connected to machine.", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.preCommand.setWhatsThis("")
#endif // QT_CONFIG(whatsthis)
        self.browsePreconnectButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Browse", None))
        self.label_username_2.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Runonce", None))
#if QT_CONFIG(tooltip)
        self.runonceCommand.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Run once command. Executed on first boot, just before UDS does anything.", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.runonceCommand.setWhatsThis("")
#endif // QT_CONFIG(whatsthis)
        self.browseRunOnceButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Browse", None))
        self.label_password_2.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Postconfig", None))
#if QT_CONFIG(tooltip)
        self.postConfigCommand.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Command to execute after UDS finalizes the VM configuration.", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.postConfigCommand.setWhatsThis("")
#endif // QT_CONFIG(whatsthis)
        self.browsePostConfigButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Browse", None))
        self.label_loglevel.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Log Level", None))

        self.tabWidget.setTabText(self.tabWidget.indexOf(self.tab_advanced), QCoreApplication.translate("UdsActorSetupDialog", u"Advanced", None))
#if QT_CONFIG(tooltip)
        self.testButton.setToolTip(QCoreApplication.translate("UdsActorSetupDialog", u"Click to test existing configuration (disabled if no config found)", None))
#endif // QT_CONFIG(tooltip)
#if QT_CONFIG(whatsthis)
        self.testButton.setWhatsThis(QCoreApplication.translate("UdsActorSetupDialog", u"<html><head/><body><p>Click on this button to test the server host and assigned toen.</p></body></html>", None))
#endif // QT_CONFIG(whatsthis)
        self.testButton.setText(QCoreApplication.translate("UdsActorSetupDialog", u"Test configuration", None))
    # retranslateUi

